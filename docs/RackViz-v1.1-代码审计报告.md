# RackViz v1.1 代码审计报告 & v1.2 升级建议

> 审计日期：2026-06-10
> 审计范围：MGBT 全项目（Rust 后端 + React 前端 + 数据库 + 配置）
> 审计版本：v1.1 (Cargo.toml version 0.2.0)

---

## 一、项目概况

| 维度 | 现状 |
|------|------|
| 后端框架 | Rust + Tauri 2.x |
| 前端框架 | React 18 + Antd 5 + Vite 6 |
| 数据库 | SQLite (rusqlite 0.32 + r2d2 连接池) |
| 日志 | flexi_logger 0.29 |
| 导入/导出 | calamine 0.25 + rust_xlsxwriter 0.79 |
| 报表 | askama 0.12 模板引擎 |
| 后端代码量 | ~15 个 Rust 源文件 |
| 前端代码量 | ~12 个 TS/TSX 源文件 |

---

## 二、架构审计

### ✅ 做得好的方面

1. **清晰的分层架构**：commands（Tauri IPC）→ db（数据访问）→ models（数据模型），职责分明
2. **连接池 + WAL 模式**：r2d2 连接池 + `PRAGMA journal_mode=WAL` + `busy_timeout`，数据库并发处理合理
3. **事务管理**：`with_transaction` 封装了 BEGIN IMMEDIATE / COMMIT / ROLLBACK
4. **前端 Hook 抽象**：`useApiList` 泛型 Hook 统一了 CRUD 操作模式
5. **useRef 稳定化**：修复了 useCallback 依赖不稳定导致的无限刷新问题
6. **乐观更新**：`useApiList` 支持 optimistic 模式，提升交互体验

### ⚠️ 架构层面问题

#### P0 — 严重问题（建议 v1.2 必修）

**1. CSP 安全策略完全关闭**
```json
"security": { "csp": null }
```
- 生产环境没有任何内容安全策略，存在 XSS 注入风险
- **建议**：配置合理的 CSP，限制 script-src、connect-src 等

**2. 数据库缺少索引**
- `devices` 表的 `serial_no`、`asset_no`、`rack_id`、`name` 字段没有索引
- 导入重复检测的三个查询（`find_device_by_serial`、`find_device_by_asset`、`find_device_by_name_in_rack`）在大数据量下会全表扫描
- 设备数量超过 1000 后性能会明显下降
- **建议**：
  ```sql
  CREATE INDEX idx_devices_serial_no ON devices(serial_no) WHERE serial_no != '';
  CREATE INDEX idx_devices_asset_no ON devices(asset_no) WHERE asset_no != '';
  CREATE INDEX idx_devices_rack_id ON devices(rack_id);
  CREATE INDEX idx_devices_name_rack ON devices(name, rack_id);
  CREATE INDEX idx_racks_room_id ON racks(room_id);
  ```

**3. 导出操作阻塞 UI 线程**
- `export_racks_excel`、`export_devices_data_excel` 等使用 `blocking_save_file()`
- 当数据量大时，UI 会在导出期间完全冻结
- **建议**：使用异步对话框 + 后台线程导出，或至少加 loading 状态提示

#### P1 — 重要问题（建议 v1.2 优先修复）

**4. COALESCE 更新模式无法置空字段**
- `update_device` 使用 `COALESCE(?1, name)` 模式
- 如果用户想把某个字段（如 `rack_id`、`department`）设为 NULL，传 `null` 会被 COALESCE 忽略
- 当前拖拽设备到资源池时，前端传 `rack_id: null`，但 COALESCE(null, rack_id) = rack_id，更新不生效
- **建议**：改用"全量更新"模式（DELETE + INSERT），或增加一个 `clear_fields` 参数标记需要置空的字段，或使用 sentinel 值

**5. 前端缺少全局错误处理**
- `tauri-api.ts` 中的 `invoke` 调用没有统一的错误拦截
- 网络异常、后端 panic、数据库错误等场景下用户只看到控制台报错
- **建议**：添加 `invoke` 包装器，统一捕获 AppError 并弹出友好提示

**6. Layout 组件过于臃肿**
- `Layout.tsx` 承担了导航、主题切换、设置弹窗、机房管理等多重职责
- `LayoutContext` 接口有 17 个属性，传递了太多状态
- **建议**：
  - 将设置弹窗拆分为独立组件 `SettingsModal`
  - 将机房管理状态抽取到 `useRoomManager` Hook
  - 简化 Context，只传递跨页面共享的核心状态

**7. 版本号不一致**
- `Cargo.toml` version = "0.2.0"
- `tauri.conf.json` version = "0.2.0"
- `Layout.tsx` 硬编码 `v1.1`
- `package.json` version = "1.0.0"
- **建议**：统一版本号管理，使用构建时注入或配置文件单一来源

---

## 三、Rust 后端代码审计

### 3.1 数据库层

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 缺少索引 | P0 | 见上文 |
| `row_to_device` 重复代码 | P2 | 同一 SELECT 列映射在 `list_devices`、`get_device`、`find_device_by_*` 三个函数中各写了一遍（约 5×3=15 行重复），建议统一为宏或函数 |
| `list_devices` 搜索仅支持 name | P1 | 搜索只匹配设备名称，不支持按 IP、序列号、资产编号搜索，实际使用场景不够 |
| `parse_optional_date` 重复定义 | P2 | `db/devices.rs` 和 `excel.rs` 各有一个 `parse_optional_date`/`parse_flexible_date`，逻辑相同 |
| 无分页支持 | P1 | `list_devices` 返回全量数据，设备上千条时前端性能差 |

### 3.2 命令层

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 连接池获取代码重复 | P2 | 每个 command 都有 `state.pool.get().map_err(\|e\| AppError::io(...))` 样板代码，建议抽取为 `DbState::conn()` 方法 |
| 缺少输入验证 | P1 | `create_device` 只验证了 name 非空和长度，缺少对 `ip_addresses` 格式、`start_u <= end_u`、U 位范围等验证 |
| 删除操作缺少级联提示 | P1 | 删除机柜时设备自动脱离，但前端没有明确告知用户哪些设备会受影响 |
| `open_log_dir` 用 `std::process::Command` | P2 | 可改用 `tauri-plugin-shell` 的 `open` 方法，更安全且跨平台一致性更好 |

### 3.3 日志系统

| 问题 | 严重度 | 说明 |
|------|--------|------|
| `reconfigure` 无法切换文件输出 | P1 | flexi_logger 的 `set_new_spec` 只能切换日志级别，不能从"仅stderr"切换到"文件+stderr"。当前关闭日志后重新开启，文件输出不会恢复 |
| 全局 Mutex 无 poisoning 处理 | P2 | `LOGGER_HANDLE.lock().unwrap()` 如果持锁线程 panic，后续所有日志操作都会 panic |
| 日志格式缺少结构化信息 | P2 | 当前日志只有文本级别+消息，缺少请求ID、操作用户等上下文，不利于排查 |

### 3.4 导入/导出

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 导入不支持更新模式 | P1 | 重复设备直接跳过，无法选择"覆盖更新"策略 |
| 导入上限硬编码 | P2 | `max_rows = 5000` 硬编码在 `import_devices_excel` 中，应可配置 |
| Excel 导出缺少样式 | P2 | 导出的 Excel 没有冻结首行、自动筛选、条件格式等，可读性一般 |
| 导入忽略机房字段 | P1 | `_room_name` 被忽略（变量名以下划线开头），导入时无法自动关联机房 |

### 3.5 安全性

| 问题 | 严重度 | 说明 |
|------|--------|------|
| CSP 关闭 | P0 | 见上文 |
| 无 SQL 注入风险 | ✅ | 所有查询使用参数化绑定 |
| 数据库文件无加密 | P2 | SQLite 数据库文件明文存储，含序列号、资产编号等敏感信息 |
| 无操作鉴权 | P2 | 所有 Tauri command 无需鉴权即可执行，适合单机场景但不适合多用户 |

---

## 四、前端代码审计

### 4.1 性能问题

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 设备列表全量渲染 | P1 | `DeviceList.tsx` 的 `filteredDevices` 在前端内存中过滤所有设备，无虚拟滚动或分页查询 |
| RackView 设备查找线性扫描 | P2 | `getDevicesForRack(rack.id)` 每次调用都 `devices.filter()`，渲染 N 个机柜时复杂度 O(N*M) |
| 无 React.memo 优化 | P2 | `DeviceDetailPanel` 等子组件未做 memo，父组件任何状态变化都会重新渲染 |
| CSS 大量内联样式 | P2 | `RackView.tsx` 中 `style={{ height: ... }}` 每次渲染生成新对象 |

### 4.2 代码质量

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 类型断言不安全 | P1 | `useDevices.ts` 中 `as Promise<Device[]>` 强制类型断言，如果后端返回格式变化会导致运行时错误 |
| 双重状态管理 | P1 | 机房管理状态既在 `Layout` 中管理，又通过 `LayoutContext` 传递，更新路径不直观 |
| 硬编码 SVG | P2 | 大量内联 SVG 代码（如 Layout.tsx 的设置图标），应抽取为 SVG 组件或使用图标库 |
| 魔数 | P2 | 机柜 U 位高度 `26px`、缩放范围 `50-200`、连接池大小 `4` 等散落在代码各处 |
| 前后端类型不同步 | P1 | `tauri-api.ts` 中的类型定义与 Rust models 是手动维护的，没有自动生成机制 |

### 4.3 UX 问题

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 设备编辑表单使用原生 date input | P2 | 浏览器原生 `<input type="date">` 样式不统一，建议用 Antd DatePicker |
| 无键盘快捷键 | P2 | 没有全局快捷键支持（虽然引入了 `tauri-plugin-global-shortcut`，但未使用） |
| 无操作撤销 | P2 | 删除、移动等操作不可撤销，用户误操作风险大 |
| 日期显示未格式化 | P2 | `DeviceDetailPanel` 中 `purchase_date` 直接显示原始日期字符串 |

---

## 五、数据库设计审计

### 5.1 表结构

```
rooms ← racks ← devices → device_models
                                    ↑
                              settings (KV)
```

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 缺少 `created_at` / `updated_at` | P1 | 所有表都没有时间戳字段，无法追踪记录创建/修改时间 |
| 缺少 `deleted_at` 软删除 | P2 | 物理删除无法恢复，审计追溯困难 |
| `settings` 表设计过于简单 | P2 | 纯 KV 结构，没有类型信息和校验，适合配置但不适合复杂设置 |
| `devices.status` 无枚举约束 | P2 | SQLite 不支持 ENUM，但可以添加 CHECK 约束 |
| 缺少数据唯一约束 | P1 | `serial_no` 和 `asset_no` 的唯一性只在代码层面检查，数据库层面没有 UNIQUE 约束 |

### 5.2 迁移策略

| 问题 | 严重度 | 说明 |
|------|--------|------|
| 迁移非原子性 | P2 | `migrate_v0_to_v1` 中先查询再执行多批 SQL，如果中途失败数据库状态不一致 |
| 无迁移回滚 | P2 | 只有 up 迁移没有 down 迁移 |
| 版本号使用 PRAGMA | ✅ | `PRAGMA user_version` 是 SQLite 推荐的版本管理方式 |

---

## 六、v1.2 升级建议路线图

### Phase 1：安全与稳定性（必须完成）

| 编号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| S-1 | 配置 CSP 安全策略 | P0 | 0.5d |
| S-2 | 添加数据库索引（v2→v3 迁移） | P0 | 0.5d |
| S-3 | 修复 COALESCE 更新无法置空字段 | P0 | 1d |
| S-4 | 修复日志 reconfigure 无法恢复文件输出 | P1 | 0.5d |
| S-5 | 添加数据库唯一约束（serial_no、asset_no） | P1 | 0.5d |

### Phase 2：功能增强（核心体验提升）

| 编号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| F-1 | 后端分页查询（list_devices 支持 offset/limit） | P1 | 1d |
| F-2 | 多字段搜索（支持 IP、序列号、资产编号） | P1 | 0.5d |
| F-3 | 导入支持"覆盖更新"模式 | P1 | 1d |
| F-4 | 导入关联机房字段（使用 room_name 匹配） | P1 | 0.5d |
| F-5 | 添加 created_at / updated_at 时间戳 | P1 | 0.5d |
| F-6 | 全局错误处理与友好提示 | P1 | 0.5d |
| F-7 | 输入验证增强（IP 格式、U 位范围、start_u ≤ end_u） | P1 | 0.5d |

### Phase 3：代码质量（技术债务清理）

| 编号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| Q-1 | 抽取 `DbState::conn()` 辅助方法 | P2 | 0.5d |
| Q-2 | 统一 `row_to_device` 映射（宏或函数） | P2 | 0.5d |
| Q-3 | 拆分 Layout 组件（SettingsModal、useRoomManager） | P2 | 1d |
| Q-4 | 前后端类型自动生成（ts-rs 或 specta） | P2 | 1d |
| Q-5 | 统一版本号管理 | P2 | 0.5d |
| Q-6 | 抽取硬编码常量到配置 | P2 | 0.5d |
| Q-7 | SVG 图标统一管理 | P2 | 0.5d |

### Phase 4：体验优化（锦上添花）

| 编号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| E-1 | Excel 导出增强（冻结首行、自动筛选、条件格式） | P2 | 1d |
| E-2 | 日期选择器改用 Antd DatePicker | P2 | 0.5d |
| E-3 | 设备列表虚拟滚动 / 虚拟表格 | P2 | 1d |
| E-4 | 操作撤销（Ctrl+Z） | P2 | 2d |
| E-5 | 键盘快捷键（利用已引入的 global-shortcut） | P2 | 1d |
| E-6 | 数据库加密（SQLCipher） | P2 | 2d |
| E-7 | 导出操作异步化 | P2 | 1d |

---

## 七、重点问题详细分析

### 7.1 COALESCE 更新模式缺陷（P0）

**现状**：
```sql
UPDATE devices SET rack_id = COALESCE(?3, rack_id) WHERE id = ?
```

**问题**：当前端传 `rack_id: null` 表示"移出机柜"时，COALESCE(null, rack_id) = rack_id，更新不生效。

**影响**：
- 拖拽设备到资源池时，前端调 `update(id, { rack_id: null })`，实际数据库中 rack_id 不变
- 编辑设备清除部门、责任人等字段时同样无法置空

**推荐方案**：采用"显式字段列表"模式
```rust
pub fn update_device(conn: &Connection, id: i32, data: &DeviceUpdate) -> Result<Option<Device>, AppError> {
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref name) = data.name {
        sets.push(format!("name = ?{}", idx));
        params.push(Box::new(name.clone()));
        idx += 1;
    }
    // rack_id 支持 None 表示置空
    if data.rack_id.is_some() {
        sets.push(format!("rack_id = ?{}", idx));
        params.push(Box::new(data.rack_id));
        idx += 1;
    }
    // ... 其他字段
}
```

或更简单地，引入 `DeviceUpdate` 中增加一个 `clear_fields: Vec<String>` 标记需要置空的字段名。

### 7.2 日志 reconfigure 问题（P1）

**现状**：`reconfigure()` 只调用 `handle.set_new_spec(spec)`，这只能改变日志级别过滤，不能改变输出目标。

**问题**：关闭日志后再开启，日志级别恢复为 Info，但文件写入器不会重新启动。

**推荐方案**：重启 Logger 而非 reconfigure
```rust
pub fn reconfigure(app: tauri::AppHandle, enabled: bool) -> Result<(), AppError> {
    // 停止当前 logger
    *LOGGER_HANDLE.lock().unwrap() = None;
    // 重新初始化
    let app_dir = ...;
    init(&app_dir, enabled)?;
    Ok(())
}
```
注意：flexi_logger 不支持完全重启，可能需要使用 `ReconfigurationHandle::set_new_spec` + 预先创建文件输出器，仅在级别间切换。或者采用 always-write-to-file + 级别控制 的策略。

### 7.3 数据库索引缺失（P0）

**现状**：`devices` 表没有任何索引（除了主键）。

**影响**：
- `find_device_by_serial` — 全表扫描 serial_no
- `find_device_by_asset` — 全表扫描 asset_no
- `find_device_by_name_in_rack` — 全表扫描 name + rack_id
- `list_devices(rack_id=Some(...))` — 全表扫描 rack_id
- 1000+ 设备时导入性能会显著下降

**推荐迁移脚本**（v3）：
```sql
CREATE INDEX IF NOT EXISTS idx_devices_serial_no ON devices(serial_no) WHERE serial_no != '';
CREATE INDEX IF NOT EXISTS idx_devices_asset_no ON devices(asset_no) WHERE asset_no != '';
CREATE INDEX IF NOT EXISTS idx_devices_rack_id ON devices(rack_id) WHERE rack_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_devices_name_rack ON devices(name, rack_id);
CREATE INDEX IF NOT EXISTS idx_racks_room_id ON racks(room_id) WHERE room_id IS NOT NULL;
PRAGMA user_version = 3;
```

---

## 八、代码质量评分

| 维度 | 评分 (1-10) | 说明 |
|------|-------------|------|
| 架构设计 | 7 | 分层清晰，但 Layout 组件职责过重 |
| 代码规范 | 7 | Rust 代码规范良好，前端有部分硬编码 |
| 错误处理 | 6 | 有 AppError 体系但前端未统一拦截 |
| 安全性 | 5 | CSP 关闭、无数据加密、无鉴权 |
| 性能 | 6 | 连接池+WAL 不错，但缺少索引和分页 |
| 可维护性 | 7 | 模块化合理，类型定义清晰 |
| 测试覆盖 | 4 | Rust 有基本单元测试，前端零测试 |
| 用户体验 | 7 | 暗色主题优秀，但缺少撤销和快捷键 |

**综合评分：6.1 / 10** — 功能完整但存在安全与性能隐患

---

## 九、总结

RackViz v1.1 作为一个从 Python 迁移到 Rust+Tauri 的项目，架构设计合理、功能基本完整。主要风险集中在：

1. **安全短板**：CSP 关闭是最大的安全隐患，生产环境必须修复
2. **数据完整性**：COALESCE 更新无法置空字段 + 数据库缺少唯一约束，会导致逻辑 Bug
3. **性能瓶颈**：缺少索引 + 无分页，设备数量增长后体验会急剧下降
4. **技术债务**：版本号不一致、类型手动同步、组件职责过重

建议 v1.2 按照 **Phase 1 → Phase 2 → Phase 3** 的顺序推进，优先解决安全与数据完整性问题，再提升功能和代码质量。预计 Phase 1+2 总工时约 5-6 天。

---

*审计报告由 RackViz v1.1 代码审计工具生成*
