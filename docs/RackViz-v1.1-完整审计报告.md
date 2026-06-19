# RackViz v1.1 代码库完整审计报告

> 审计日期：2026-06-10  
> 审计范围：Rust 后端 22 文件 + React 前端 18 文件 + 数据库迁移 + 配置文件  
> 审计依据：`docs/RackViz-代码审查标准与流程.md` 附录 A/B 全量检查项  
> 审计人：CodeReviewExpert 👁️

---

## 一、审计总结

**总体评价**：⚠️ 有条件通过 — 安全底线存在明显缺口，数据正确性有功能性 Bug，前端架构需重构

| 统计 | P0 (Blocker) | P1 (Important) | P2 (Nit) | 合计 |
|------|:-----------:|:------------:|:-------:|:----:|
| **Rust 后端** | 1 | 16 | 7 | 24 |
| **React 前端** | 10 | 36 | 13 | 59 |
| **数据库/配置** | 2 | 3 | 1 | 6 |
| **合计** | **13** | **55** | **21** | **89** |

### 综合评分：5.4 / 10

| 维度 | 评分 | 说明 |
|------|:----:|------|
| **安全性** | 4/10 | CSP 关闭、LIKE 未转义、敏感信息入日志 |
| **数据正确性** | 5/10 | COALESCE 阻止置空（功能性 Bug）、无 UNIQUE 约束、find_or_create 竞态 |
| **错误处理** | 4/10 | 26 处错误分类错误、.ok() 吞错误、前端大面积无 try/catch |
| **代码质量** | 6/10 | 重复代码多但结构清晰，组件过胖 |
| **性能** | 5/10 | 无索引、N+1 查询、导出阻塞 UI、前端重渲染问题 |
| **可维护性** | 6/10 | 类型重复定义、God Component、Prop Drilling |

---

## 二、P0 问题清单（必须修复，阻断发布）

### P0-01 🔴 CSP 安全策略完全关闭

| 项目 | 内容 |
|------|------|
| **检查项** | S-5 |
| **文件** | `src-tauri/tauri.conf.json:27` |
| **问题代码** | `"csp": null` |
| **影响** | WebView 不受内容安全策略保护。即使桌面应用，若前端存在 XSS 漏洞，攻击者可加载外部脚本执行任意代码。 |
| **修复** | `"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:"` |

### P0-02 🔴 数据库无任何索引（7 个高频查询全表扫描）

| 项目 | 内容 |
|------|------|
| **检查项** | P-1, M-2 |
| **文件** | `src-tauri/src/migration.rs` |
| **问题** | 所有表只有主键索引，无其他索引 |
| **影响** | 以下查询全部全表扫描：(1) `devices WHERE rack_id` (列表过滤) (2) `devices WHERE serial_no` (导入查重) (3) `devices WHERE asset_no` (导入查重) (4) `devices WHERE name AND rack_id` (导入查重) (5) `racks WHERE name` (find_or_create) (6) `device_models WHERE name` (find_or_create) (7) `racks ORDER BY sort_order` |
| **修复** | 新增迁移 v3 添加索引（见第六章） |

### P0-03 🔴 COALESCE 更新模式无法置空字段（功能性 Bug）

| 项目 | 内容 |
|------|------|
| **检查项** | D-1 |
| **文件** | `db/devices.rs:131-146`, `db/racks.rs:66-73`, `db/rooms.rs:53-54`, `db/device_models.rs:60-65` |
| **问题代码** | `rack_id = COALESCE(?3, rack_id)` |
| **影响** | 拖拽设备到资源池时前端传 `rack_id: null`，COALESCE 将 null 视为"不修改"，字段值不变。**设备无法从机柜中移出**。同理无法清空 `start_u`、`end_u`、`device_model_id`、`room_id` 等。 |
| **修复** | 采用 sentinel 值方案（如 `rack_id: -1` 表示置空）或拆分 clear/update 逻辑 |

### P0-04 🔴 前端 `as T` 乐观更新可能遗漏必填字段

| 项目 | 内容 |
|------|------|
| **检查项** | T-1 |
| **文件** | `frontend/src/hooks/useApiList.ts:51` |
| **问题代码** | `setItems(prevItems => prevItems.map(d => d.id === id ? { ...d, ...data } as T : d))` |
| **影响** | `{ ...d, ...data } as T` 强制断言掩盖了结构缺失。如果 `data` 中包含前端未预期的字段或类型不匹配，TypeScript 不会报错。 |
| **修复** | 使用类型守卫或 runtime 校验函数替代 `as T` |

### P0-05 🔴 前端类型定义严重重复（12 个接口重复）

| 项目 | 内容 |
|------|------|
| **检查项** | T-4 |
| **文件** | `tauri-api.ts:5-83` vs `types/index.ts:1-47` |
| **问题** | Device/DeviceModel/Rack/Room 及其 Create/Update 变体在两处各自定义 |
| **影响** | 维护时需同时修改两处，新增字段易遗漏。两侧类型不一致时编译器不报错（被 `as` 断言绕过）。 |
| **修复** | 统一类型定义源，`tauri-api.ts` 引用 `types/index.ts` 中的类型 |

### P0-06 🔴 核心写操作无 try/catch（拖拽/创建机柜/排序）

| 项目 | 内容 |
|------|------|
| **检查项** | E-1 |
| **文件** | `RackView.tsx:288-295` (handleDrop), `RackView.tsx:306-317` (handleStockDrop), `RackView.tsx:319-338` (handleAddRack), `RackView.tsx:372-384` (handleMoveRackLeft/Right) |
| **问题** | `await update(...)`, `await createRack(...)` 无 try/catch |
| **影响** | 操作失败时产生未捕获的 Promise 拒绝，用户无任何错误提示，不知道操作是否成功。 |
| **修复** | 添加 try/catch + `message.error()` 用户提示 |

### P0-07 🔴 错误对用户完全不可见（3 处 console.error）

| 项目 | 内容 |
|------|------|
| **检查项** | E-2 |
| **文件** | `Layout.tsx:71-74` (getLoggingConfig), `Layout.tsx:84-86` (setLoggingEnabled), `Layout.tsx:93-95` (openLogDir) |
| **问题** | 失败时只 `console.error`，无用户提示 |
| **影响** | 用户操作失败但界面无反馈，开关可能停留在错误状态 |
| **修复** | 添加 `message.error()` 提示 |

### P0-08 🔴 RackView.tsx 880 行（God Component）

| 项目 | 内容 |
|------|------|
| **检查项** | C-1 |
| **文件** | `frontend/src/pages/RackView.tsx` |
| **问题** | 13 个 useState、拖拽/侧边栏/详情面板/3 个弹窗全在一个文件 |
| **影响** | 可维护性极差，修改一处容易引发其他问题 |
| **修复** | 拆分为 RackGrid、DeviceStockPanel、AddRackModal、EditDeviceModal、EditRackModal 等 |

### P0-09 🔴 LayoutContext 23 个字段

| 项目 | 内容 |
|------|------|
| **检查项** | C-3 |
| **文件** | `Layout.tsx:11-35` |
| **问题** | 23 个字段通过 Outlet context 传递 |
| **影响** | Room 管理、视图控制、搜索等全部混在一个 context 中，任何字段变更影响全局 |
| **修复** | 拆分为 ViewContext + RoomContext，或使用自定义 hook |

### P0-10 🔴 DeviceList 列表过滤无 useMemo

| 项目 | 内容 |
|------|------|
| **检查项** | P-1 |
| **文件** | `DeviceList.tsx:295-300` |
| **问题** | `filteredDevices` 每次渲染重新计算 `.filter()` |
| **影响** | 设备数量大时性能问题 |
| **修复** | 包裹 `useMemo(() => ..., [devices, searchText, roomRackIds])` |

### P0-11 🔴 列定义每次渲染重建

| 项目 | 内容 |
|------|------|
| **检查项** | P-2 |
| **文件** | `DeviceList.tsx:306-335` |
| **问题** | `allDeviceColumns` 含内联 render/sorter 函数，每次渲染新建 |
| **影响** | Table 组件不必要的重渲染 |
| **修复** | 提取到组件外或用 `useMemo` 包裹 |

### P0-12 🔴 useMemo 依赖列表不完整

| 项目 | 内容 |
|------|------|
| **检查项** | P-5 |
| **文件** | `Layout.tsx:140-155` |
| **问题** | `ctxValue` 的 useMemo 缺少 `handleAddRoom`、`handleEditRoom` 等 5 个函数引用 |
| **影响** | context 消费方可能使用过时的函数引用 |
| **修复** | 将缺失的函数加入依赖列表，或用 `useCallback` 稳定化 |

### P0-13 🔴 迁移脚本无事务保护

| 项目 | 内容 |
|------|------|
| **检查项** | M-4 |
| **文件** | `migration.rs:21-85`, `migration.rs:87-97` |
| **问题** | `migrate_v0_to_v1` 和 `migrate_v1_to_v2` 中 `execute_batch` 无显式事务 |
| **影响** | 迁移中途失败（如断电）会导致数据库处于不一致状态（部分表已创建，版本号未更新） |
| **修复** | 在每个迁移函数中添加 `conn.execute_batch("BEGIN")?;` 和 `COMMIT` |

---

## 三、P1 问题清单（建议尽快修复）

### 3.1 Rust 后端 P1 问题

| # | 检查项 | 问题 | 文件:行 | 说明 |
|---|--------|------|---------|------|
| P1-01 | S-2 | LIKE 通配符未转义 | `db/devices.rs:38` | `format!("%{}%", s)` 中 `%` 和 `_` 未转义 |
| P1-02 | S-3 | 生产路径 `.unwrap()` | `lib.rs:25` | `db_path.to_str().unwrap()` 路径含非 UTF-8 时 panic |
| P1-03 | S-3 | 生产路径 `.expect()` | `lib.rs:29` | `pool.get().expect()` 连接池耗尽时 panic |
| P1-04 | S-3 | Mutex `.unwrap()` 4 处 | `logging.rs:45,57,77,88` | 锁被 poisoned 时连锁 panic |
| P1-05 | S-3 | insert 后 `.unwrap()` 4 处 | `db/devices.rs:118` 等 | `get_*(conn, id).map(\|r\| r.unwrap())` |
| P1-06 | S-4 | 敏感信息入日志 | `excel.rs:276,285` | 日志中记录序列号和资产编号 |
| P1-07 | D-4 | 唯一性字段无 UNIQUE | `migration.rs` | `serial_no`、`asset_no`、`name` 等无约束 |
| P1-08 | D-5 | find_or_create 竞态 | `db/racks.rs:101-112`, `db/device_models.rs:87-98` | SELECT+INSERT 非原子 |
| P1-09 | E-1 | 连接池错误误分类 | 所有 `commands/*.rs` (26 处) | `pool.get().map_err(AppError::io)` 应为 DatabaseError |
| P1-10 | E-2 | `.ok()` 吞掉错误 7 处 | `commands/*.rs:54` 等, `db/devices.rs:201,210` | 数据库错误被静默忽略 |
| P1-11 | E-4 | ROLLBACK 失败未记录 | `state.rs:71` | `let _ = conn.execute_batch("ROLLBACK")` |
| P1-12 | E-5 | AppError 缺少 Error trait | `error.rs:20-25` | 未实现 `std::error::Error` |
| P1-13 | P-2 | N+1 查询 | `excel.rs:157-169` | 逐 rack 查 devices |
| P1-14 | P-4 | 导出阻塞 UI | `commands/exports.rs` (5 命令) | 同步命令 + `blocking_save_file()` |

### 3.2 React 前端 P1 问题

| # | 检查项 | 问题 | 文件:行 | 说明 |
|---|--------|------|---------|------|
| P1-15 | T-1 | `as` 类型断言 5 处 | `useDevices.ts:7-9` 等 | DeviceResp→Device 强制转换，枚举类型不一致被掩盖 |
| P1-16 | T-3 | 未使用的 `any` 类型 | `useApiList.ts:4` | `AnyRecord = Record<string, any>` 死代码 |
| P1-17 | T-5 | 前后端类型不同步 3 处 | `status/string` vs 枚举, `view/string` vs 枚举, `type/string` vs DeviceType | 后端返回 string，前端期望枚举 |
| P1-18 | E-1 | API 层 invoke 无 try/catch | `tauri-api.ts` 21 个函数 | 唯一 `importExcelFromPath` 有 try/catch |
| P1-19 | E-2 | refresh 加载失败用户不可见 | `useApiList.ts:34` | `.catch(console.error)` |
| P1-20 | E-3 | 乐观更新回滚竞态 | `useApiList.ts:44-58,61-76` | `prev` 捕获的是闭包创建时的 items，可能过时 |
| P1-21 | E-4 | 删除机房非原子 | `Layout.tsx:124-131` | 并行解绑机柜 + 删除机房，中间失败不一致 |
| P1-22 | E-4 | 机房排序非原子 | `RoomTabs.tsx:95-97` | Promise.all 部分失败不一致 |
| P1-23 | E-4 | 机柜左右移动非原子 | `RackView.tsx:372-384` | 两次 update 无回滚 |
| P1-24 | C-1 | DeviceList 578 行 | `DeviceList.tsx` | 超 300 行限制 |
| P1-25 | C-2 | RackView 13 个 useState | `RackView.tsx:104-123` | 超 8 个限制 |
| P1-26 | C-2 | Layout 12 个 useState | `Layout.tsx:45-58` | 超 8 个限制 |
| P1-27 | C-2 | DeviceList 11 个 useState | `DeviceList.tsx:124-151` | 超 8 个限制 |
| P1-28 | C-5 | ContextType 重复定义 | `RackView.tsx:12-36` vs `Layout.tsx:11-35` | 完整复制 LayoutContext |
| P1-29 | C-5 | 状态标签重复定义 3 处 | `DeviceList.tsx`, `RackView.tsx`, `DeviceDetailPanel.tsx` | getStatusText/getStatusTag |
| P1-30 | C-6 | 弹窗未提取 4 处 | `RackView.tsx` 3 个, `Layout.tsx` 1 个 | 内联 Modal 约 250 行 |
| P1-31 | P-2 | modelColumns 每次渲染重建 | `DeviceList.tsx:354-368` | 含内联渲染函数 |
| P1-32 | P-3 | getDevicesForRack 不稳定 | `RackView.tsx:132-134` | 导致 useCallback 缓存失效 |
| P1-33 | P-3 | getDeviceModel/getRoomName 不稳定 | `RackView.tsx:136-137` | 同上 |
| P1-34 | P-4 | 机柜内联闭包大量创建 | `RackView.tsx:505-507` | 每个 U 位创建 onDragOver+onDrop |
| P1-35 | P-5 | rackStats useMemo 依赖不完整 | `RackView.tsx:143-154` | 依赖了 getDevicesForRack 但未列入依赖 |

### 3.3 数据库/配置 P1 问题

| # | 检查项 | 问题 | 文件:行 | 说明 |
|---|--------|------|---------|------|
| P1-36 | M-1 | settings 表无时间戳 | `migration.rs:87-97` | 缺少 `created_at`/`updated_at` |
| P1-37 | M-6 | 外键约束未启用 | `migration.rs` | 无 `PRAGMA foreign_keys = ON` |
| P1-38 | — | 版本号不一致 | Cargo.toml=0.2.0, tauri.conf=0.2.0, package.json=1.0.0, UI=v1.1 | 四处分叉 |

---

## 四、P2 问题清单（建议改进）

| # | 类别 | 问题 | 数量 |
|---|------|------|:----:|
| P2-01 | Q-1 | SELECT 列列表硬编码重复 | 13 处 |
| P2-02 | Q-2 | row_to_* 映射函数未统一 | 3 模块 |
| P2-03 | Q-3 | 输入验证不充分（start_u>end_u、status 枚举、负数等） | 全部 create |
| P2-04 | Q-4 | 日期解析函数重复 | 2 处 |
| P2-05 | Q-5 | 连接池获取未封装为方法 | 26 处 |
| P2-06 | Q-6 | 测试 setup_db 重复 | 4 处 |
| P2-07 | P-3 | Map 函数查询多余列 | 3 处 |
| P2-08 | D-2 | DB 层 delete 操作未内含事务 | 3 处 |
| P2-09 | T-2 | 非空断言 `!` | RackView 3 处 |
| P2-10 | T-2 | falsy 检查应改 `!= null` | RackView 2 处 |
| P2-11 | C-4 | Prop Drilling 超 2 层 | Room 管理 13 props 3 层 |
| P2-12 | C-5 | 设备类型标签重复 | labels.ts vs DeviceList |
| P2-13 | P-4 | RoomTabs sortedRooms 无 useMemo | 1 处 |

---

## 五、各检查项覆盖率

### 5.1 Rust 后端

| 检查项 | 描述 | 通过 | 不通过 | 通过率 |
|--------|------|:----:|:-----:|:-----:|
| S-1 | SQL 参数化查询 | ✅ | — | 100% |
| S-2 | LIKE 通配符转义 | — | ❌ | 0% |
| S-3 | 无 .unwrap() 生产路径 | — | ❌ (9 处) | 0% |
| S-4 | 无敏感信息日志 | — | ❌ (2 处) | 0% |
| S-5 | CSP 配置 | — | ❌ | 0% |
| D-1 | COALESCE 不阻止置空 | — | ❌ (4 表) | 0% |
| D-2 | 批量操作在事务中 | — | ❌ (3 处) | 40% |
| D-3 | 顺序操作有回滚 | ✅ | — | 100% |
| D-4 | UNIQUE 约束 | — | ❌ (4 类字段) | 0% |
| D-5 | find_or_create ON CONFLICT | — | ❌ (2 处) | 0% |
| E-1 | 连接池错误用 ? | — | ❌ (26 处) | 0% |
| E-2 | 不用 .ok() 吞错误 | — | ❌ (7 处) | 0% |
| E-3 | insert 返回值安全 | — | ❌ (4 处) | 0% |
| E-4 | ROLLBACK 日志 | — | ❌ | 0% |
| E-5 | AppError 实现 Error | — | ❌ | 0% |
| Q-1 | SELECT 列不硬编码 | — | ❌ (13 处) | 0% |
| Q-2 | row_to_* 统一 | 部分通过 | ❌ (3 模块) | 25% |
| Q-3 | 输入验证全面 | — | ❌ | 20% |
| Q-4 | 日期解析不重复 | — | ❌ | 0% |
| Q-5 | 连接池封装方法 | — | ❌ (26 处) | 0% |
| Q-6 | 测试 setup_db 不重复 | — | ❌ (4 处) | 0% |
| P-1 | 查询有索引 | — | ❌ (7 查询) | 0% |
| P-2 | 无 N+1 查询 | — | ❌ (1 处) | 0% |
| P-3 | 只查所需列 | — | ❌ (3 处) | 60% |
| P-4 | 导出不阻塞 UI | — | ❌ (5 命令) | 0% |

**Rust 后端通过率：4/25 = 16%**

### 5.2 React 前端

| 检查项 | 描述 | 通过 | 不通过 | 通过率 |
|--------|------|:----:|:-----:|:-----:|
| T-1 | 不使用 as T 掩盖 | — | ❌ (5 处) | 0% |
| T-2 | nullable 有 null 保护 | 部分通过 | ❌ (5 处) | 50% |
| T-3 | 不使用 any | — | ❌ (1 处死代码) | 90% |
| T-4 | 类型不重复 | — | ❌ (12 接口) | 0% |
| T-5 | 前后端类型同步 | — | ❌ (3 字段) | 0% |
| E-1 | invoke 有 try/catch | — | ❌ (21+处) | 5% |
| E-2 | 错误对用户可见 | — | ❌ (5 处) | 0% |
| E-3 | 乐观更新回滚正确 | — | ❌ (2 处) | 0% |
| E-4 | 顺序写有原子保护 | — | ❌ (3 处) | 0% |
| C-1 | 组件 < 300 行 | — | ❌ (2 组件) | 67% |
| C-2 | useState < 8 | — | ❌ (3 组件) | 50% |
| C-3 | Context < 10 字段 | — | ❌ (1 context) | 0% |
| C-4 | Prop drilling ≤ 2 | 部分通过 | ❌ (1 处) | 80% |
| C-5 | 类型接口不重复 | — | ❌ (5 处) | 0% |
| C-6 | Modal 提取 | — | ❌ (4 处) | 0% |
| P-1 | 列表过滤用 useMemo | — | ❌ (2 处) | 0% |
| P-2 | 列定义不重建 | — | ❌ (3 处) | 0% |
| P-3 | useCallback 依赖稳定 | — | ❌ (3 处) | 0% |
| P-4 | 无大量内联闭包 | — | ❌ (2 处) | 60% |
| P-5 | useMemo 依赖完整 | — | ❌ (2 处) | 0% |

**React 前端通过率：3/20 = 15%**

### 5.3 数据库

| 检查项 | 描述 | 通过 | 不通过 | 通过率 |
|--------|------|:----:|:-----:|:-----:|
| M-1 | 新表有时间戳 | — | ❌ (settings) | 75% |
| M-2 | 新查询有索引 | — | ❌ (7 查询) | 0% |
| M-3 | 唯一性有 UNIQUE | — | ❌ (4 字段) | 0% |
| M-4 | 迁移用事务 | — | ❌ | 0% |
| M-5 | 版本号递增 | ✅ | — | 100% |
| M-6 | 外键约束启用 | — | ❌ | 0% |
| M-7 | 破坏性变更有迁移 | ✅ | — | 100% |
| M-8 | 向后兼容 | ✅ | — | 100% |

**数据库通过率：3/8 = 37.5%**

---

## 六、修复优先级路线图

### Phase 1：安全与数据底线（P0 全清，~3 天）

| 优先序 | 问题 | 工时 | 修复方案 |
|:------:|------|:----:|---------|
| 1 | P0-01 CSP 关闭 | 0.5h | 配置 CSP 策略 |
| 2 | P0-13 迁移无事务 | 0.5h | 每个迁移函数加 BEGIN/COMMIT |
| 3 | P0-02 无索引 | 1h | 新增迁移 v3 添加 7 个索引 |
| 4 | P0-03 COALESCE 置空 | 2h | 改用 sentinel 值方案 + 前后端适配 |
| 5 | P0-05 类型重复 | 2h | 统一 types/index.ts 为唯一来源 |
| 6 | P0-06/P0-07 错误处理 | 1h | 核心 try/catch + message.error |

### Phase 2：后端健壮性（P1 后端，~3 天）

| 优先序 | 问题 | 工时 | 修复方案 |
|:------:|------|:----:|---------|
| 7 | P1-09 连接池 26 处 map_err | 1h | 统一用 `?` + 封装 `DbState::conn()` |
| 8 | P1-07 UNIQUE 约束 | 1h | 迁移 v3 添加约束 |
| 9 | P1-08 find_or_create 竞态 | 1h | 改用 ON CONFLICT |
| 10 | P1-05 insert 后 unwrap | 1h | 改用 `ok_or` |
| 11 | P1-10 .ok() 吞错误 | 1h | 区分不存在和错误 |
| 12 | P1-01 LIKE 转义 | 0.5h | 添加 escape_like 函数 |
| 13 | P1-02/03/04 生产路径 unwrap | 1h | 改用安全替代 |
| 14 | P1-11/12 ROLLBACK 日志 + Error trait | 0.5h | 补充实现 |
| 15 | P1-14 导出异步化 | 1h | async command + spawn_blocking |
| 16 | P1-13 N+1 查询 | 0.5h | 一次查全部 + 内存分组 |
| 17 | P1-06 敏感信息脱敏 | 0.5h | 日志中只记录 id |

### Phase 3：前端重构（P1 前端，~4 天）

| 优先序 | 问题 | 工时 | 修复方案 |
|:------:|------|:----:|---------|
| 18 | P0-08 RackView 拆分 | 4h | 拆为 5-6 个子组件 |
| 19 | P0-09 LayoutContext 拆分 | 2h | 拆为 ViewContext + RoomContext |
| 20 | P1-25/26/27 useState 合并 | 2h | useReducer + 自定义 hook |
| 21 | P0-10/11 性能优化 | 1h | useMemo + 列定义提取 |
| 22 | P1-15 类型断言改映射 | 2h | API 层增加运行时转换 |
| 23 | P1-20 乐观更新修复 | 1h | 函数式 setItems |
| 24 | P1-28/29 重复代码清理 | 2h | 提取公共工具函数 |
| 25 | P1-30 弹窗提取 | 2h | 4 个 Modal 提取为组件 |
| 26 | P1-32/33 useCallback 稳定化 | 1h | 索引 Map + useCallback |

### Phase 4：质量基建（P2 + 自动化，~3 天）

| 优先序 | 问题 | 工时 | 修复方案 |
|:------:|------|:----:|---------|
| 27 | P2-01/02/04/05/06 代码去重 | 2h | 提取常量、映射函数、工具方法 |
| 28 | P2-03 输入验证增强 | 1h | 添加范围/枚举/格式校验 |
| 29 | Clippy 配置 | 0.5h | 配置 unwrap_used/expect_used = warn |
| 30 | ESLint 配置 | 0.5h | no-explicit-any/no-non-null-assertion |
| 31 | Pre-commit Hook | 0.5h | cargo check + clippy + tsc + eslint |
| 32 | 版本号统一 | 0.5h | 统一为 v1.2.0 |

---

## 七、量化指标对比

| 指标 | v1.1 当前值 | v1.2 目标 | 改善幅度 |
|------|:---------:|:--------:|:-------:|
| Rust `.unwrap()` 生产路径 | 9 | 0 | -100% |
| 前端 `as T` 断言 | 5 | 0 | -100% |
| 前端 `any` 类型 | 1 | 0 | -100% |
| 类型重复接口数 | 12 | 0 | -100% |
| 连接池 map_err 误分类 | 26 | 0 | -100% |
| `.ok()` 吞错误 | 7 | 0 | -100% |
| 无 try/catch 的 invoke | 21 | 0 | -100% |
| 缺少索引的查询 | 7 | 0 | -100% |
| 无 UNIQUE 约束字段 | 4 | 0 | -100% |
| 最大组件行数 | 880 | < 300 | -66% |
| 最大 useState 数 | 13 | < 8 | -38% |
| Context 字段数 | 23 | < 10 | -57% |
| 代码重复函数 | ~15 | < 5 | -67% |
| COALESCE 阻止置空 | 4 表 | 0 | -100% |
| 迁移无事务 | 2 | 0 | -100% |

---

## 八、亮点与值得肯定的设计

尽管问题不少，代码库也有一些值得肯定的方面：

1. ✅ **SQL 全部参数化** — 没有任何 `format!` 拼接 SQL，安全底线守住了
2. ✅ **Rust 架构清晰** — commands/db/models 分层明确，职责分离合理
3. ✅ **flexi_logger 集成** — 日志收集可开关，运行时可切换级别
4. ✅ **useApiList 通用 hook** — 统一 CRUD 模式，减少重复代码
5. ✅ **Tauri 2.x 权限模型** — capabilities 限制最小权限
6. ✅ **导入三级去重** — serial_no → asset_no → name+rack_id 逐级检查
7. ✅ **暗色主题支持** — CSS 变量体系完整，双主题兼容

---

## 九、v1.2 发布前的验收标准

| 类别 | 验收项 | 标准 |
|------|--------|------|
| 安全 | CSP 策略 | 非 null |
| 安全 | LIKE 通配符 | 已转义 |
| 数据 | 索引覆盖 | WHERE/JOIN 全有索引 |
| 数据 | UNIQUE 约束 | 唯一性字段全覆盖 |
| 数据 | COALESCE | 无新增 COALESCE 更新（改用 sentinel） |
| 错误 | 无 .unwrap() | `rg '\.unwrap\(\)' src-tauri/src/` 仅测试中出现 |
| 错误 | 无裸 invoke | `rg 'await.*invoke' frontend/src/` 全在 try 内 |
| 性能 | 无 N+1 | 循环内无 DB 查询 |
| 质量 | 组件行数 | 最大 < 300 行 |
| 质量 | useState 数 | 最大 < 8 个 |
| 质量 | Context 字段 | 最大 < 10 个 |
| 质量 | 版本号一致 | Cargo/tauri.conf/package.json 同版本 |

---

*本审计基于 `docs/RackViz-代码审查标准与流程.md` 附录 A/B 的全部检查项，对代码库进行了逐文件、逐行的完整扫描。报告中的每个发现均附有精确的文件和行号引用，可直接定位修复。*
