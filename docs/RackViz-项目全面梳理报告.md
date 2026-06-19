# RackViz 项目全面梳理报告

> 撰写日期：2026-06-19
> 数据来源：实际代码扫描 + 审计报告 + 升级方案 + 项目总结
> 项目版本：v1.1 → v1.2 升级中

---

## 一、项目概况

### 1.1 基本信息

| 项目 | 内容 |
|------|------|
| **项目名称** | RackViz — 数据中心机柜可视化管理桌面应用 |
| **代码库名** | MGBT |
| **技术栈** | Rust + Tauri 2.x + React 18 + TypeScript + Ant Design 5 + SQLite |
| **当前版本** | v1.1（Tauri 重构版） |
| **升级目标** | v1.2，代码质量从 5.4/10 → 8.0/10 |
| **团队** | 1 人（Jinkela，Owner） |
| **仓库地址** | https://github.com/JinkelaD/RackViz.git |

### 1.2 项目历史

```
2026-05-27  ● 立项 — Python FastAPI + pywebview 初始版本
2026-05-28  ● v1.0 完成 — 机柜可视化 + 设备台账
2026-06-09  ● 重构评估 — 架构对比、方案设计
2026-06-10  ● v1.1 发布 — Tauri 2.x 重构完成
2026-06-19  ● v1.2 启动 — 代码质量升级进行中
```

### 1.3 核心价值

| 价值维度 | 描述 |
|----------|------|
| **机柜 U 位可视化** | 42U 网格拖拽上下架，直观替代表格 |
| **设备全生命周期** | 入库→上架→运维→下架→退役，全程追溯 |
| **Excel 双向互通** | 导入台账/导出报表，兼容现有工作流 |
| **轻量高性能** | 安装包 < 20MB，启动 < 500ms，内存 < 60MB |

### 1.4 性能对比（v1.0 → v1.1）

| 指标 | v1.0 (Python) | v1.1 (Tauri) | 提升 |
|------|:---:|:---:|:---:|
| 安装包 | 100+ MB | 18 MB | 🚀 82%↓ |
| 启动速度 | 3-5s | < 500ms | 🚀 6-10x |
| 运行内存 | 150-240 MB | 30-50 MB | 🚀 80%↓ |
| 数据库并发 | Mutex 串行 | r2d2 连接池(4) | 🚀 4x |

---

## 二、代码规模统计

### 2.1 Rust 后端（21 文件，2183 行）

| 文件 | 行数 | 职责 |
|------|:---:|------|
| `db/devices.rs` | 296 | 设备 CRUD（**最大文件**） |
| `excel.rs` | 323 | Excel 导入导出 |
| `models.rs` | 167 | 数据模型定义 |
| `db/racks.rs` | 160 | 机柜 CRUD |
| `db/device_models.rs` | 136 | 设备型号 CRUD |
| `db/rooms.rs` | 116 | 机房 CRUD |
| `error.rs` | 110 | AppError 错误类型 |
| `commands/exports.rs` | 107 | 导出命令 |
| `migration.rs` | 97 | 数据库迁移 v0→v1→v2 |
| `logging.rs` | 96 | flexi_logger 日志配置 |
| `report.rs` | 89 | HTML 报告生成 |
| `lib.rs` | 76 | Tauri 插件注册+启动 |
| `state.rs` | 75 | AppState(DbPool)+事务 |
| `commands/devices.rs` | 65 | 设备命令 |
| `commands/settings.rs` | 65 | 设置命令 |
| `commands/racks.rs` | 53 | 机柜命令 |
| `commands/rooms.rs` | 53 | 机房命令 |
| `commands/device_models.rs` | 53 | 型号命令 |
| `db/settings.rs` | 27 | 设置 KV 存储 |
| `main.rs` | 5 | 入口 |
| `commands/mod.rs` + `db/mod.rs` | 14 | 模块声明 |

### 2.2 React 前端（19 文件，约 4210 行）

| 文件 | 行数 | 职责 |
|------|:---:|------|
| `styles/global.css` | 2265 | 全局样式（**最大文件**） |
| `pages/RackView.tsx` | 879 | 机柜可视化 🔴 超 300 行红线 |
| `pages/DeviceList.tsx` | 577 | 设备台账 🔴 超 300 行红线 |
| `tauri-api.ts` | 268 | IPC 封装层 |
| `components/Layout.tsx` | 252 | 全局布局 |
| `components/RoomTabs.tsx` | 163 | 机房标签页 |
| `components/DeviceDetailPanel.tsx` | 138 | 设备详情面板 |
| `main.tsx` | 109 | 入口 |
| `hooks/useApiList.ts` | 79 | 通用 CRUD hook |
| `contexts/ThemeContext.tsx` | 45 | 主题切换 |
| `types/index.ts` | 49 | 前端类型定义 |
| `components/StatusBar.tsx` | 87 | 状态栏 |
| `App.tsx` | 16 | 路由入口 |
| `hooks/useDevices.ts` | 14 | 设备数据 hook |
| `hooks/useRacks.ts` | 19 | 机柜数据 hook |
| `hooks/useRooms.ts` | 13 | 机房数据 hook |
| `hooks/useDeviceModels.ts` | 13 | 型号数据 hook |
| `constants/labels.ts` | 7 | 标签常量 |

### 2.3 文档（6 份）

| 文档 | 内容 |
|------|------|
| `RackViz-v1.1-产品介绍.md` | 功能说明与技术特性 |
| `RackViz-v1.1-代码审计报告.md` | 第一轮审计发现 |
| `RackViz-v1.1-完整审计报告.md` | 全量 89 个发现（P0×13 + P1×55 + P2×21） |
| `RackViz-代码审查标准与流程.md` | 19 条审查红线 |
| `RackViz-v1.2-升级方案.md` | 6 Phase 升级路线图 |
| `RackViz-项目阶段性总结.md` | 从立项到 v1.1 的复盘 |

---

## 三、红线违规实际扫描结果

> 基于**实际代码逐行扫描**，而非仅参考审计报告。

### 3.1 Rust 后端红线违规（共 63 处）

| 违规项 | 数量 | 严重性 | 关键位置 |
|--------|:---:|:---:|----------|
| **COALESCE 更新模式** | 4 文件全部 update | 🔴 P0 | `db/devices.rs:131-146`, `db/racks.rs:66-73`, `db/rooms.rs:53-54`, `db/device_models.rs:60-65` |
| **`.unwrap()` 生产路径** | 9 处 | 🔴 P0 | `lib.rs:25`, `logging.rs:45,57,77,88`, `db/*.rs` insert 后 4 处 |
| **`.expect()` 生产路径** | 4 处 | 🔴 P0 | `lib.rs:26,29,35,75` |
| **连接池错误分类错误** | ~27 处 | 🔴 P0 | 所有 `commands/*.rs` — `pool.get().map_err(AppError::io)` |
| **LIKE 未转义 `%` `_`** | 1 处 | 🟡 P1 | `db/devices.rs:37-38` |
| **`.ok()` 吞错误** | 12+ 处 | 🟡 P1 | `db/settings.rs:9`, `db/devices.rs:201,210`, `commands/*.rs` 4 处, `excel.rs` 5 处 |
| **日志含敏感信息** | 2 处 | 🔴 P0 | `excel.rs:276,285` — 明文记录 `serial_no`/`asset_no` |
| **format!() 拼接 SQL** | 3 处 | 🟡 P1 | `db/devices.rs:32,37,50`（参数化绑定但模式违规） |
| **迁移脚本无事务** | 2 处 | 🔴 P0 | `migration.rs` 两个迁移函数无显式 BEGIN/COMMIT |
| **无索引** | 7 个查询全表扫描 | 🔴 P0 | devices WHERE rack_id/serial_no/asset_no/name+rack_id 等 |
| **无 UNIQUE 约束** | 4 个字段 | 🟡 P1 | serial_no, asset_no, name(racks), name(device_models) |

### 3.2 React 前端红线违规（共 60+ 处）

| 违规项 | 数量 | 严重性 | 关键位置 |
|--------|:---:|:---:|----------|
| **`as T` 类型断言** | 18 处 | 🔴 P0 | 4 个 hooks(13处) + useApiList.ts:51 + DeviceList.tsx(4处) + tauri-api.ts:243 |
| **invoke 无 try/catch** | 28/29 裸调用 | 🔴 P0 | `tauri-api.ts` 仅 1 个有 try/catch |
| **组件超 300 行** | 2 个 | 🔴 P0 | `RackView.tsx:879行`, `DeviceList.tsx:577行` |
| **useState 超 8 个** | 3 个 | 🔴 P0 | `RackView.tsx:13个`, `Layout.tsx:12个`, `DeviceList.tsx:11个` |
| **过滤/排序无 useMemo** | 3 处 | 🔴 P0 | `DeviceList.tsx:295,302`, `RoomTabs.tsx:103` |
| **列定义每次重建** | 4 处 | 🔴 P0 | `DeviceList.tsx:306-368` 全部列定义 |
| **`!` 非空断言** | 4处(6断言点) | 🟡 P1 | `RackView.tsx:517-519`(start_u/end_u!), `main.tsx:99` |
| **错误仅 console.error** | 5 处 | 🔴 P0 | `useApiList.ts:34`, `Layout.tsx:74,85,95`, `DeviceList.tsx:216` |
| **类型定义重复** | 4 组 | 🔴 P0 | `types/index.ts` ↔ `tauri-api.ts` — Device/DeviceModel/Rack/Room 各两套 |
| **`any` 类型** | 1 处 | 🟡 P1 | `useApiList.ts:4`（死代码 AnyRecord） |
| **LayoutContext 23 字段** | 1 处 | 🔴 P0 | `Layout.tsx:11-35` |

### 3.3 数据库红线违规

| 违规项 | 状态 | 位置 |
|--------|:---:|------|
| **迁移脚本无事务** | 🔴 | `migration.rs` 两个迁移函数 |
| **unique 字段无 UNIQUE 约束** | 🔴 | `migration.rs` 定义的所有表 |
| **外键字段无索引** | 🔴 | `migration.rs` |
| **迁移脚本无回滚方案** | 🔴 | `migration.rs` |
| **外键约束未启用** | 🟡 | 无 `PRAGMA foreign_keys = ON` |
| **settings 表无时间戳** | 🟡 | `migration.rs:87-97` |

---

## 四、综合质量评分

| 维度 | 评分 | 说明 |
|------|:---:|------|
| **安全性** | 4/10 | CSP 关闭、LIKE 未转义、敏感信息入日志、26 处连接池错误分类为 IoError |
| **数据正确性** | 5/10 | COALESCE 阻止置空（功能性 Bug）、无 UNIQUE 约束、find_or_create 竞态 |
| **错误处理** | 4/10 | 9 处 unwrap、4 处 expect、12+ 处 .ok() 吞错、28 个裸 invoke、5 处 console.error |
| **代码质量** | 6/10 | 4 组类型重复→18 处 as 断言、RackView 879 行 God Component |
| **性能** | 5/10 | 无索引、N+1 查询、导出阻塞 UI、前端无 useMemo、列定义每次重建 |
| **可维护性** | 6/10 | LayoutContext 23 字段、Prop Drilling、重复代码 ~15 处 |
| **综合** | **5.4/10** | 目标 8.0/10 |

### 各层通过率

| 层 | 检查项数 | 通过数 | 通过率 |
|----|:---:|:---:|:---:|
| Rust 后端 | 25 | 4 | 16% |
| React 前端 | 20 | 3 | 15% |
| 数据库 | 8 | 3 | 37.5% |

---

## 五、功能现状

### 5.1 已完成功能（v1.1）

| 功能 | 状态 | 说明 |
|------|:---:|------|
| 机柜 U 位可视化 | ✅ | 42U 网格，颜色编码 |
| 设备拖拽上下架 | ✅ | HTML5 Drag & Drop |
| 设备台账 CRUD | ✅ | 增删改查 + 搜索 |
| Excel 导入导出 | ✅ | 含三重查重 |
| 机房/机柜/设备型号管理 | ✅ | 基础 CRUD |
| HTML 报告导出 | ✅ | 报告生成 |
| 日志收集系统 | ✅ | flexi_logger + UI 开关 |
| 操作审计 | ✅ | 所有 CRUD 操作自动记录 |
| 明暗主题切换 | ✅ | ThemeContext |

### 5.2 Tauri IPC 命令清单（28 个）

| 模块 | 命令数 | 命令 |
|------|:---:|------|
| rooms | 5 | list/get/create/update/delete |
| racks | 5 | list/get/create/update/delete |
| devices | 5 | list/get/create/update/delete |
| device_models | 5 | list/get/create/update/delete |
| exports | 5 | export_racks_excel/export_devices_data_excel/export_single_rack_excel/export_report_html/import_excel_from_path |
| settings | 3 | get_logging_config/set_logging_enabled/open_log_dir |

### 5.3 数据库表（5 张）

| 表 | 字段数 | 关键字段 |
|----|:---:|----------|
| rooms | 4 | id, name, location, sort_order |
| racks | 8 | id, name, height_u, row, col, view, sort_order, room_id |
| devices | 16 | id, name, serial_no, asset_no, rack_id, start_u, end_u, ip_addresses, status, ... |
| device_models | 6 | id, name, manufacturer, type, height_u, power_watt |
| settings | 2 | key, value |

---

## 六、升级路线图

### 6.1 v1.2 升级总览

| Phase | 内容 | 工期 | 核心任务数 | 状态 |
|-------|------|:---:|:---:|:---:|
| **Phase 0** | 自动化配置 + 版本号统一 | 0.5d | 6 | ⏳ 待开始 |
| **Phase 1** | 安全与数据底线（P0 全清） | 3d | 16 | ⏳ 待开始 |
| **Phase 2** | 后端健壮性（P1 后端） | 3d | 10 | ⏳ 待开始 |
| **Phase 3** | 前端重构（P1 前端） | 4d | 22 | ⏳ 待开始 |
| **Phase 4** | 功能增强 | 8d | 19（可选） | ⏳ 待开始 |
| **Phase 5** | 质量基建 | 3d | 8 | ⏳ 待开始 |

**总工期**：13 天（核心）+ 8 天（功能增强）= **21 天**

### 6.2 Phase 1 关键修复清单（P0 全清）

| 优先序 | 问题 | 工时 | 修复方案 |
|:---:|------|:---:|---------|
| 1 | P0-01 CSP 关闭 | 0.5h | `tauri.conf.json` 添加 CSP 策略 |
| 2 | P0-13 迁移无事务 | 0.5h | 每个 migrate_* 加 BEGIN/COMMIT |
| 3 | P0-02 无索引 | 1h | 新增迁移 v3 添加 7 个索引 |
| 4 | P0-03 COALESCE 置空 | 2h | 改用 sentinel 值方案 + 前后端适配 |
| 5 | P0-05 类型重复 | 2h | 统一 types/index.ts 为唯一来源 |
| 6 | P0-06/07 错误处理 | 1h | 核心 try/catch + message.error |
| 7 | P0-08 RackView 拆分 | 4h | 拆为 5-6 个子组件 |
| 8 | P0-09 LayoutContext | 2h | 拆为 ViewContext + RoomContext |
| 9 | P0-10/11 性能优化 | 1h | useMemo + 列定义提取 |

---

## 七、亮点与值得肯定的设计

1. ✅ **SQL 全部参数化** — 没有任何 format! 直接拼接用户输入到 SQL 值部分
2. ✅ **Rust 架构清晰** — commands/db/models 分层明确，职责分离合理
3. ✅ **flexi_logger 集成** — 日志收集可开关，运行时可切换级别
4. ✅ **useApiList 通用 hook** — 统一 CRUD 模式，减少重复代码
5. ✅ **导入三级去重** — serial_no → asset_no → name+rack_id 逐级检查
6. ✅ **暗色主题支持** — CSS 变量体系完整，双主题兼容
7. ✅ **批量导入事务保护** — `db::with_transaction` 包裹导入流程
8. ✅ **r2d2 连接池 + WAL 模式** — 并发性能远超 Python v1.0

---

## 八、风险评估

| 风险 | 级别 | 说明 | 应对 |
|------|:---:|------|------|
| P0 未清零 | 🔴 | 13 个安全/数据底线问题 | Phase 1 优先，3 天内清零 |
| COALESCE 修复回归 | 🟡 | 拖拽功能可能失效 | 充分单元测试 + 手动测试 |
| 前端拆分引入 Bug | 🟡 | RackView 拆分后功能异常 | 逐组件拆分 + 每步测试 |
| 单人开发 | 🟡 | 1 人承担全部开发 | AI 协作补充，关键决策需复盘 |
| Phase 3 超预期 | 🟡 | 前端重构可能延期 2-3 天 | 优先级排序，非关键可延后 |

---

## 九、下一步行动建议

| 优先级 | 行动 | 预期结果 |
|--------|------|----------|
| **P0** | 创建 develop 分支，开始 Phase 0 | 自动化配置就绪 |
| **P0** | 执行 Phase 1：修复所有 P0 问题 | 安全底线达标 |
| **P1** | 执行 Phase 2：后端健壮性修复 | Rust 通过率 > 80% |
| **P1** | 执行 Phase 3：前端重构 | 前端通过率 > 80% |
| **P2** | Phase 4 功能增强（可选） | 用户体验提升 |
| **P2** | Phase 5 质量基建 | 自动化检查 + CI |

---

*报告完毕。建议立即开始 Phase 0（自动化配置 + 版本号统一），然后进入 Phase 1 清零所有 P0 问题。*
