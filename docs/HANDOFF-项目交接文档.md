# RackViz 项目交接文档

> **文档性质**：供下游 Agent 直接导入的项目交接材料。目标是让一个从未接触本项目的 Agent 在 10 分钟内建立完整上下文，并知道去哪读、改什么、避什么坑。
> **生成时间**：2026-09-02　|　**代码快照**：2026-07-27（源码自该日期后未变更）
> **项目路径**：`D:\Cursor Project\RackViz`（Windows / Git Bash）
>
> **配套详参**（本文件的展开版，按需读取）：
> - `docs/analysis-架构分析报告.md` —— 1,182 行，源码级架构/模块/契约/质量审计
> - `docs/analysis-磁盘占用与清理方案.md` —— 471 行，磁盘实测数据与逐项清理风险评估
>
> **⚠️ v1.2 状态更新（2026 后续会话）**：本文档所描述的 v1.1 缺陷清单已按
> `RackViz-v1.2-升级方案.md` 的 Phase 1–5 修复落地（详见 `docs/RackViz-v1.2-落地记录.md`）。
> 其中「COALESCE 无法置空」「CSP 关闭」「LIKE 未转义」「无索引/约束」「导出阻塞 UI」
> 等 P0/P1 问题均已修复；前端 RackView/DeviceList 已完成组件化拆分（各 < 300 行）。

---

## 一、项目概述

### 1.1 一句话定位

**RackViz 是一个单机 Windows 桌面应用，用拖拽可视化替代 Excel 台账，管理数据中心"哪台设备在哪个机柜的哪个 U 位"。**

### 1.2 目标用户与痛点

| 维度 | 内容 |
|------|------|
| 用户 | 数据中心 / 机房运维人员、资产管理员 |
| 痛点 | Excel 台账无法直观表达空间关系；改一次装机位置要在表格里手动改多个字段；机柜容量靠人脑计算；找不到空闲 U 位 |
| 解法 | 机柜 2D 可视化 + 拖拽上下架 + U 位自动分配 + 使用率预警 + Excel 双向导入导出 + HTML 报表 |
| 部署形态 | 纯本地单机，**无网络、无服务端、无鉴权、无多用户** |

### 1.3 规模与成熟度

| 指标 | 数值 |
|------|------|
| Rust 后端 | 22 个 `.rs` 文件，**2,183 行**（含约 202 行单元测试） |
| 前端 TS/TSX | **2,728 行** |
| 前端 CSS | 2,265 行（单个 `global.css`） |
| 文档 | 3 份原始文档 + 2 份本次分析文档 |
| **磁盘占用** | **1.8 GB**（其中 1.6 GB 是 Rust 构建缓存，项目本体不足 2 MB） |
| 成熟度 | **功能 = v1.1 完成态；v1.2 升级方案几乎一行未落地** |

### 1.4 架构健康度评分

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构清晰度 | ⭐⭐⭐⭐⭐ | 分层干净，新人 1 天可懂全貌 |
| 代码规模 | ⭐⭐⭐⭐⭐ | 小，刚从 Python 重写，无历史包袱 |
| 功能完整度 | ⭐⭐⭐⭐ | v1.1 功能都在位，2 个导出缺 UI 入口 |
| 正确性 | ⭐⭐ | 🔴 1 个 P0 数据 Bug，无索引无唯一约束 |
| 安全性 | ⭐⭐ | CSP 关闭、LIKE 未转义、权限面偏大 |
| 可维护性 | ⭐⭐⭐ | 2 个 God Component、23 字段 Context、无 lint |
| 测试覆盖 | ⭐⭐ | 仅 db 层 23 个用例，命令层/前端零测试 |

---

## 二、技术架构

### 2.1 技术栈

**前端**
| 类别 | 选型 | 版本 | 备注 |
|------|------|------|------|
| 框架 | React | ^18.3.1 | 纯函数组件 + Hooks |
| 语言 | TypeScript | ^5.6.3 | `strict: true` |
| 构建 | Vite | ^6.4.0 | 端口 5173，`strictPort: true` |
| UI 库 | Ant Design | ^5.22.0 | Table/Modal/Form/Select/Tag |
| 图表 | **无第三方库** | — | 机柜视图是手写 DOM + CSS 绝对定位 |
| 路由 | react-router-dom | ^6.28.0 | `BrowserRouter`，仅 2 个页面 |
| 状态管理 | **无** | — | 仅 React Context + Hooks |
| Tauri 桥接 | @tauri-apps/api | ^2.11.0 | 只用 `invoke` |

**后端**
| Crate | 版本 | 用途 |
|-------|------|------|
| `tauri` | 2 | 桌面框架（`custom-protocol`） |
| `rusqlite` | 0.32 | SQLite（`bundled` 静态编译，无需系统依赖） |
| `r2d2` + `r2d2_sqlite` | 0.8 / 0.25 | 连接池，`max_size(4)` |
| `calamine` | 0.25 | 读 xlsx |
| `rust_xlsxwriter` | 0.79 | 写 xlsx |
| `askama` | 0.12 | 编译期 HTML 模板，渲染报表 |
| `chrono` / `serde` / `log` + `flexi_logger` | — | 日期 / 序列化 / 日志（按天轮转保留 7 份） |
| ⚠️ 无用依赖 | — | `tauri-plugin-shell`（完全未用）、`tauri-plugin-fs`（未用）、`dirs`（未用）、`tauri-build` 误入 `[dependencies]` |

### 2.2 分层架构

```
┌─────────────────────────────────────────────────────────┐
│ 前端 React (frontend/src/)                               │
│   pages/RackView.tsx      机柜可视化主页面（879 行）      │
│   pages/DeviceList.tsx    设备台账表格（577 行）          │
│   hooks/useApiList.ts     通用 CRUD Hook（乐观更新）      │
│   tauri-api.ts            ★ 28 个 invoke 封装（契约核心） │
└────────────────────┬────────────────────────────────────┘
                     │ invoke()  JSON over IPC
┌────────────────────▼────────────────────────────────────┐
│ commands/*.rs    参数校验（仅 name 非空/长度）+ log::info │
│                  rooms / racks / devices / device_models │
│                  exports(4导出+1导入) / settings(日志)   │
└────────────────────┬────────────────────────────────────┘
┌────────────────────▼────────────────────────────────────┐
│ db/*.rs          纯 SQL，参数化查询，COALESCE 部分更新    │
└────────────────────┬────────────────────────────────────┘
┌────────────────────▼────────────────────────────────────┐
│ rusqlite ← r2d2 池(4) ← SQLite (WAL 模式)                │
│ 旁支：excel.rs / report.rs 直接调 db 层，不经 commands    │
└─────────────────────────────────────────────────────────┘
```

### 2.3 数据模型

```
rooms (机房)                    device_models (型号字典)
  │ 1                                  │ 1
  │ room_id  ON DELETE SET NULL        │ device_model_id  ON DELETE SET NULL
  ▼ N                                  ▼ N
racks (机柜) ──── 1:N ────► devices (设备)
  height_u (默认 42U)          start_u / end_u  (U 位闭区间，可 NULL)
  row / col / view / sort_order
```

- **全部外键 `ON DELETE SET NULL` 软级联** —— 删机房不删机柜，删机柜不删设备，只解除关联（刻意设计："设备不丢失"）
- **表**：`rooms` / `racks` / `devices` / `device_models` / `settings`（KV，目前只存 `logging_enabled`）
- **关键字段语义**
  - `devices.status`：`online` / `offline` / `unconfigured`（在资源池里）
  - `racks.view`：`front` / `rear`，前端按此过滤
  - `device_models.type`：`server`/`switch`/`router`/`storage`/`security`，决定配色与图标
  - `start_u` / `end_u`：**闭区间**，1U 设备 `start_u == end_u`；为 NULL 表示未上架
  - `sort_order`：机房与机柜的排序依据，前端拖拽排序就是批量改这个字段
- **迁移机制**：`PRAGMA user_version`，`v0→v1`（建 4 表）→ `v1→v2`（建 settings），`CURRENT_VERSION = 2`。🔴 **无事务保护**，中途失败会留半截 schema
- **数据库位置**：`%LOCALAPPDATA%\com.rackviz.app\rackviz.db`（`lib.rs:20-22`）；日志在同级 `logs/rackviz.log`

### 2.4 前后端契约：28 个 Tauri 命令

统一约定：所有命令返回 `Result<T, AppError>`，`AppError` 序列化为 `{ code, message, detail? }`。

| 分组 | 命令 |
|------|------|
| 设备（5） | `list_devices` / `get_device` / `create_device` / `update_device` / `delete_device` |
| 机柜（5） | `list_racks` / `get_rack` / `create_rack` / `update_rack` / `delete_rack` |
| 机房（5） | `list_rooms` / `get_room` / `create_room` / `update_room` / `delete_room` |
| 型号（5） | `list_device_models` / `get_device_model` / `create/update/delete_device_model` |
| 导入导出（5） | `export_racks_excel` / `export_devices_data_excel` / `export_single_rack_excel` / `export_report_html` / `import_excel_from_path` |
| 设置（3） | `get_logging_config` / `set_logging_enabled` / `open_log_dir` |

**注意点**
- 4 个 `get_*` 命令（device/rack/room/device_model）**前端均未调用**，属预留接口
- `export_racks_excel`（机柜部署图）、`export_single_rack_excel`（单机柜）**后端已实现但前端未接线**，加按钮即可启用
- 4 个导出命令在 Rust 侧调 `blocking_save_file()` 弹原生对话框并**阻塞当前线程**；产物经 `std::fs::write` 落盘（不让二进制走 IPC）
- 导入是唯一的两段式调用：前端先用 dialog 插件选文件 → 再把路径传给 `import_excel_from_path`
- `LoggingConfig` 是**唯一用 camelCase 的响应结构**（`log_dir`），其余全 snake_case
- 前端目前**几乎不解析 `code`**，多数只是 `console.error`

### 2.5 关键 Tauri 配置

```jsonc
{
  "productName": "RackViz", "version": "0.2.0", "identifier": "com.rackviz.app",
  "build": {
    "frontendDist": "../frontend/dist",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "cd ../frontend && npm run build"
  },
  "app": {
    "windows": [{ "title": "RackViz - 机房设备管理系统", "width": 1400, "height": 900,
                  "minWidth": 1024, "minHeight": 680, "dragDropEnabled": false }],
    "security": { "csp": null }          // 🔴 P0：CSP 关闭
  },
  "bundle": { "active": true, "targets": ["msi", "nsis"] }
}
```

> ⚠️ **`dragDropEnabled: false` 是刻意设计** —— 否则 WebView 会抢走 HTML5 拖拽（设备上下架的核心交互）。改动前三思。
> ⚠️ **改 CSP 前必须先处理 `index.html:9` 的 Google Fonts CDN**，否则字体被阻断。

---

## 三、关键代码位置说明

### 3.1 目录树（源码部分）

```
D:\Cursor Project\RackViz\
├── .gitignore                  # 已忽略 target/ node_modules/ dist/ *.db
├── build-exe.bat               # Windows 一键打包脚本
├── docs/                       # 文档（禁止删除）
├── frontend/
│   ├── src/                    # ★ 前端源码，18 个文件（禁止删除）
│   │   ├── main.tsx            # React 入口 + AntD 主题 token
│   │   ├── App.tsx             # 路由表（2 个页面）
│   │   ├── tauri-api.ts        # ★★ 28 个 invoke 封装 + 类型（契约核心）
│   │   ├── types/index.ts      # 领域类型
│   │   ├── constants/labels.ts # 设备类型中文映射
│   │   ├── contexts/ThemeContext.tsx
│   │   ├── hooks/              # useApiList.ts ★★ / useDevices / useRacks / useRooms / useDeviceModels
│   │   ├── components/         # Layout / RoomTabs / StatusBar / DeviceDetailPanel
│   │   ├── pages/              # RackView.tsx ★★(879行) / DeviceList.tsx(577行)
│   │   └── styles/global.css   # 2,265 行
│   ├── package.json / package-lock.json   # ★★ lock 禁止删除
│   ├── dist/                   # 构建产物（1.1M，可删）
│   └── node_modules/           # 依赖（172M，可删）
└── src-tauri/
    ├── Cargo.toml / Cargo.lock # ★★ lock 禁止删除（锁 500 个包）
    ├── tauri.conf.json / capabilities/default.json / build.rs
    ├── icons/                  # 应用图标（打包必需）
    ├── gen/schemas/            # Tauri 自动生成的 ACL schema（勿手改）
    ├── templates/report.html   # Askama 报表模板
    ├── target/                 # ⚠️ 1.6 GB 构建缓存（可删，本次分析焦点）
    └── src/                    # ★ Rust 源码，22 个文件（禁止删除）
        ├── main.rs             # 5 行入口
        ├── lib.rs              # ★★ Tauri Builder + 28 个命令注册
        ├── models.rs           # ★★ 13 个数据结构
        ├── error.rs            # AppError 统一错误 + 6 个 From 转换
        ├── state.rs            # r2d2 连接池 + with_transaction
        ├── migration.rs        # 迁移 v0→v1→v2
        ├── logging.rs          # flexi_logger 初始化与运行时重配
        ├── excel.rs            # ★★ Excel 导入(三重查重)/导出
        ├── report.rs           # Askama 报表渲染
        ├── commands/           # ★★ IPC 层：mod / rooms / racks / devices / device_models / exports / settings
        └── db/                 # 数据访问层：mod / rooms / racks / devices / device_models / settings
```

### 3.2 后端改动索引

| 想改什么 | 文件:行 | 说明 |
|----------|---------|------|
| **注册新命令** | `lib.rs:44-73` | `tauri::generate_handler![...]` 加一行 |
| 注册 Tauri 插件 | `lib.rs:16-18` | `.plugin(tauri_plugin_xxx::init())` |
| 改数据库路径 | `lib.rs:20-22` | `app_local_data_dir().join("rackviz.db")` |
| 加/改数据表 | `migration.rs:21/87/3/10` | 新增迁移需**四处同步**：新函数 + `CURRENT_VERSION` + `run()` match + 老分支串联 |
| 加数据库索引 | `migration.rs` | 需新增 `migrate_v2_to_v3()`（**当前 0 个索引**） |
| 改设备 CRUD SQL | `db/devices.rs:27/69/90/121/216` | list / get / insert / update / delete |
| 🔴 **改 U 位置空逻辑** | `db/devices.rs:134-136` | `rack_id = COALESCE(?3, rack_id)` ← P0 Bug 源头 |
| 改机柜/机房/型号 SQL | `db/racks.rs:5/24/43/60/83`、`db/rooms.rs:5/20/35/48/60`、`db/device_models.rs:5/22/39/54/75` | |
| 改设备搜索逻辑 | `db/devices.rs:27-67` | 动态拼 WHERE；目前只搜 `name`，且 LIKE 未转义 |
| 改导入列映射 | `excel.rs:232-270` | 按**列序号** 0-14 读取，无表头识别 |
| 改导入查重规则 | `excel.rs:272-296` | 三重查重：序列号 / 资产编号 / 同机柜同名 |
| 让导入关联机房 | `excel.rs:242` | `let _room_name = ...` ← 读取后丢弃 |
| 改导出行数上限 | `excel.rs:222` | `max_rows = 5000u32` |
| 改 N+1 查询 | `excel.rs:159` | `for rack in racks` 循环内调 `list_devices` |
| 改 HTML 报表 | `report.rs:28-89` + `templates/report.html` | Askama 模板 |
| 改日志轮转策略 | `logging.rs:36-40` | `Cleanup::KeepLogFiles(7)` |
| 改连接池大小 | `state.rs:34` | `.max_size(4)` |
| 改 SQLite PRAGMA | `state.rs:17-21` | WAL / `foreign_keys=ON` / `busy_timeout=5000` |
| 改事务行为 | `state.rs:60-75` | `with_transaction()`：BEGIN IMMEDIATE / COMMIT / ROLLBACK |
| 加错误码 | `error.rs:6-16` + `:27-44` | 枚举变体 + 构造器 |
| 改参数校验 | `commands/devices.rs:25-30`（name 非空 + ≤100）；`racks.rs:21-23` 等（仅 name 非空） | |
| 改导出对话框 | `commands/exports.rs:24-29` 等 4 处 | `blocking_save_file()`，v1.2 要求改异步 |

### 3.3 前端改动索引

| 想改什么 | 文件:行 | 说明 |
|----------|---------|------|
| **加新页面/路由** | `App.tsx:8-15` | 在 `<Route element={<Layout/>}>` 下加子路由 |
| 加导航标签 | `Layout.tsx:99-102` | `navItems` 数组 |
| 改 AntD 主题色 | `main.tsx:9-43` | `darkToken` / `lightToken`（两套都要改） |
| **加/改后端调用** | `tauri-api.ts` | 28 个函数，每个一行 `invoke()` |
| 改类型定义 | `types/index.ts` + `tauri-api.ts:5-139` | ⚠️ **两处需同步**（12 个重复接口） |
| **改通用 CRUD 行为** | `hooks/useApiList.ts` | `refresh`/`create`/`update`/`remove` 全在这 |
| 改乐观更新策略 | `useApiList.ts:50-58`(update)、`:67-75`(remove) | 回滚逻辑有竞态 |
| **改 U 位分配算法** | `RackView.tsx:38-94` | `findAvailableSlot()` —— 业务核心算法 |
| 改机柜渲染 | `RackView.tsx:436-591` | header / body / grid / devices / footer |
| 改 U 位格子 | `RackView.tsx:494-512` | `u = height_u - i`（自顶向下） |
| 改设备块定位 | `RackView.tsx:517-518` | `(end_u - start_u + 1) * 26` px |
| 改拖拽上架 | `RackView.tsx:256-296` | `handleDrop` |
| 改拖回资源池 | `RackView.tsx:306-317` | `handleStockDrop` 🔴 受 COALESCE Bug 影响 |
| 改缩放范围 | `Layout.tsx:60-62` | 50–200%，步长 10% |
| 改 U 位预警阈值 | `RackView.tsx:439`、`StatusBar.tsx:82` | `> 85` 变红 |
| 改台账列定义 | `DeviceList.tsx:306-335` | `allDeviceColumns` 11 列 |
| 改列宽拖拽 | `DeviceList.tsx:42-89` | `ResizableTitle` 纯手写 mousemove |
| 改导入按钮 | `DeviceList.tsx:161-181` | |
| 改导出菜单 | `DeviceList.tsx:396-410` | 目前只有 Excel / HTML 两项 |
| 改机房拖拽排序 | `RoomTabs.tsx:62-101` | 重排 `sort_order` 并批量 update |
| 改全局样式 | `styles/global.css` | token `:7-144`，组件样式 `:186` 起 |

### 3.4 构建与配置

| 想改什么 | 文件:行 |
|----------|---------|
| 改窗口尺寸/标题 | `tauri.conf.json:13-24` |
| 改打包目标 | `tauri.conf.json:30-48` |
| **改 CSP**（P0） | `tauri.conf.json:27` |
| 改 Tauri 权限 | `capabilities/default.json`（建议收紧 `shell:default`） |
| 改 Vite 端口 | `vite.config.ts:8`（需与 `tauri.conf.json:8` 的 `devUrl` 一致） |
| 改代码分包 | `vite.config.ts:18-21` |
| 改 Rust 依赖 | `Cargo.toml`（可清理 `tauri-plugin-shell` / `tauri-plugin-fs` / `dirs` / dependencies 段的 `tauri-build`） |
| 改打包脚本 | `build-exe.bat:109`（当前 `cargo build --release`，只出裸 exe；要安装包需改成 `cargo tauri build`） |

---

## 四、磁盘占用分析与清理结果

> 详参：`docs/analysis-磁盘占用与清理方案.md`（含完整复现命令）

### 4.1 实测总览

| 路径 | 大小 | 占项目 |
|------|------|--------|
| **项目总计** | **1.8 G** | 100% |
| `src-tauri/target/` | **1.6 G** | ≈89% |
| `frontend/node_modules/` | 172 M | ≈9% |
| `frontend/dist/` | 1.1 M | <0.1% |
| **源码 + 文档 + 配置（本体）** | **< 2 M** | <0.2% |

> **关键观察**：1.8 G 中 **99.9% 是可再生的构建产物与依赖**。

### 4.2 `target/release` 的 1.6 G 是怎么来的

**目录级（`du`，硬链接已去重）**

| 子目录 | 大小 | 占比 | 说明 |
|--------|------|------|------|
| `deps/` | **1.4 G** | 87.5% | 所有 crate 的编译产物 |
| `build/` | **194 M** | 12.2% | 131 个构建脚本的产物与运行输出 |
| `.fingerprint/` | 2.7 M | 0.17% | 584 个编译单元的指纹元数据 |
| `incremental/` | **0** | 0% | 空目录（release 默认关闭增量编译） |
| `examples/` | 0 | 0% | 空目录 |

**文件类型（表观大小 1773.4 MiB，未去重）**

| 类型 | 大小 | 占比 |
|------|------|------|
| `.rlib` | 779.9 MiB | 44.0% |
| `.rmeta` | 449.5 MiB | 25.4% |
| `.pdb` | 290.7 MiB | 16.4% |
| `.exe` | 132.8 MiB | 7.5% |
| `.dll` | 70.4 MiB | 4.0% |
| `.lib` / `.a` / `.o` / 其余 | ~50 MiB | 2.7% |

**归因解读**
- `windows` 172 MiB + `windows-sys` 108 MiB = **280 MiB（占 deps 20.4%）** —— Tauri 在 Windows 上的固有代价（Win32 全量绑定），无法避免
- Tauri 全家桶（tauri / tauri_utils / tauri_macros / tauri_build / webview2_com_sys）≈ **210 MiB**
- 本项目自身 `rackviz_lib` 70.4 MiB
- `.rlib` + `.rmeta` 合计 **69.3%** —— Rust 依赖编译的固有开销，**要么整体存在（增量可用），要么整体消失（全量重编），无法选择性压缩**

**`.pdb` 调试符号专项**：135 个，名义 290.7 MiB。因大量硬链接（`build_script_build.exe` 成对、根 `rackviz.exe/pdb` 指向 `deps`），**实际占盘约 190–210 MiB**。

**最终产物**：仅 `rackviz.exe` **17,867,776 字节（17.0 MiB）**。🔴 **无 `.msi` / `.nsis`**（`bundle/` 目录不存在——`build-exe.bat` 只跑 `cargo build --release`，从不触发打包）。

**根因**：`Cargo.toml` **无 `[profile.release]` 段**，fingerprint 记录 `rustflags: []` → 使用 Cargo 默认值，`strip` 未启用，故 MSVC 为各编译单元生成了符号文件。

### 4.3 能否安全删除？—— 判断结论

| 问题 | 结论 | 依据 |
|------|------|------|
| **能否删** | ✅ **技术上能删，100% 可再生** | `Cargo.lock` 锁 500 个包；`~/.cargo/registry/src` 有 1.4 G 依赖源码，**离线即可重编** |
| **是否需要完全重编** | ⚠️ **是，且是 100% 全量** | `incremental/` 为空（无增量缓存）；584 个编译单元；`deps/` 里的 `.rlib` 就是依赖的编译产物，删了必须全部重编 |
| **重编耗时** | **约 10–20 分钟** | 依据：`build-exe.bat` 作者实测注释"首次编译需要 10-20 分钟"；依赖已本地缓存，省去下载时间 |
| **会不会影响已安装程序** | ❌ **不会** | 实测：无 `rackviz` 进程运行；`Program Files` 与 `AppData` 下**均无已安装副本**；`bundle/` 不存在。exe 是"原地产物"而非"拷贝出去的安装副本" |
| **会不会丢失不可再生的内容** | ❌ **不会** | `target/` 下 100% 可由 `Cargo.lock` + 源码 + registry 确定性推导 |
| **唯一的真实风险** | ⚠️ **无 Git 兜底** | 项目根**无 `.git` 目录**，删除不可回滚；且本机唯一的 `rackviz.exe` 会消失 |
| **是否存在"陈旧产物"可部分清理** | ❌ **不存在** | 用 `.fingerprint` 的 584 个 hash 反查 `deps/` 的 1409 个文件：**0 个孤儿**。156 个"多 hash"crate 全部是合法的 build-dependencies host 单元或 feature 变体 |

### 4.4 逐项清理清单

#### 🟢 档位 A：立刻可安全删除（低风险）

| 路径 | 释放 | 风险 | 恢复方式 | 推荐 |
|------|------|------|----------|:---:|
| `frontend/node_modules/` | **172 M** | 低 | `cd frontend && npm install` | ✅ **强推** |
| `frontend/dist/` | 1.1 M | 低 | `cd frontend && npm run build` | ✅ 顺手 |
| `target/release/**/*.pdb`（135 个） | ≈190–210 MiB | 低 | 不会自动补回，需 `touch src/main.rs && cargo build --release` | ⚠️ 有保留 |

> **关于删 `.pdb` 的保留意见**：这是"手工切 target 内部"的非标准操作，Cargo 不保证支持。收益约 200 MiB（占项目 11%），但会失去崩溃堆栈符号化能力。若后续还要调试本机崩溃，**建议保留**。

#### 🟡 档位 B：可删但需重建（耗时 / 需网络）

| 路径 | 释放 | 风险 | 理由 |
|------|------|------|------|
| `src-tauri/target/`（整体） | **1.6 G** | 中 | 全可再生，但全量重编 584 单元（10–20 分钟），且本机唯一 exe 消失 |
| `%LOCALAPPDATA%\npm-cache` | 618 M | 中 | 项目外、全局共享，清了会让所有 npm 项目安装变慢 |

#### 🚫 档位 C：绝对不要动

| 路径 | 大小 | 理由 |
|------|------|------|
| `frontend/src/`、`src-tauri/src/`、`docs/` | ≈2 M | **源码与文档，后续 agent 必须读** |
| **`frontend/package-lock.json`** | 99 K | 锁版本。`package.json` 用 `^` 范围，删后 `npm install` 会解析到新版本可能不兼容；**无 git 不可恢复** |
| **`src-tauri/Cargo.lock`** | 126 K | 锁 **500 个包**。删后 cargo 重新解析，Tauri 2.x 生态下极易编译失败；**无 git 不可恢复** |
| `src-tauri/gen/schemas/` | 957 K | Tauri 生成的 ACL schema，capabilities 校验与 IDE 补全依赖它 |
| `src-tauri/icons/` | 292 K | 打包必需（`tauri.conf.json` 引用） |
| **数据库 `%LOCALAPPDATA%\com.rackviz.app\rackviz.db`** | — | 🔴 **用户数据，删了全没** |
| **`~/.cargo/registry`（1.6 G）** | 1.6 G | 🚫 **全局共享**。删了影响机器上所有其他 Rust 项目，需重新下载 1.4 G；且**本项目重编正需要它提供离线依赖** |
| `src-tauri/target/.fingerprint/` | 2.7 M | 单独删 = 触发全量重编，等于删了整个 target 却只省 2.7 M |

#### 已排查但无可清理项

- `frontend/node_modules/.vite`、`.cache` —— **均不存在**
- 全项目（排除 target / node_modules）搜索 `*.log` `*.bak` `*.old` `*~` `.DS_Store` `Thumbs.db` `*.tmp` `*.tsbuildinfo` —— **零命中**
- `src-tauri/target/debug` —— **不存在**（从未 debug 构建）
- `frontend/public/` —— 空目录
- `icons/`、`public/` —— 无超大资源文件

### 4.5 方案对比与最终建议

| 维度 | 🟢 保守 | 🟡 推荐折中 | 🔴 激进 |
|------|---------|-----------|---------|
| 操作 | 删 `.pdb` + `node_modules` + `dist` | **只删 `node_modules` + `dist`** | 删 `target/` + `node_modules` + `dist` |
| **释放** | ≈364–384 MiB | **≈174 MiB** | **≈1.77 G**（1.8G → 2M） |
| Rust 重编成本 | 0 | **0** | 全量 584 单元（10–20 分钟） |
| 丢失 exe 风险 | 无 | **无** | ⚠️ **有**（本机唯一 exe） |
| 丢失调试符号 | 是 | 否 | 是 |
| 影响源码阅读 | 否 | 否 | 否 |

**→ 最终建议（按执行顺序）：**

1. **先执行**（纯收益、零 Rust 成本）：
   ```bash
   cd "D:/Cursor Project/RackViz/frontend" && rm -rf node_modules dist
   ```
   释放 173 M；`package-lock.json` 在位保证版本一致；不影响任何源码，不影响 `target/` 增量。

2. **`target/` 的 1.6 G 建议保留到最后再决定**。它的唯一真实价值是"省一次 10–20 分钟的全量重编"+"随时能跑 exe"。后续其他 agent 很可能需要反复 `cargo build` 验证改动——留着它等于用 1.6 G 磁盘换后续每次编译的分钟级加速。

3. **如果磁盘确实吃紧必须删 `target/`**，请**先备份 exe 到项目外**（本项目无 `.git`，删了就真没了）：
   ```bash
   mkdir -p "D:/Cursor Project/RackViz_backup"
   cp "D:/Cursor Project/RackViz/src-tauri/target/release/rackviz.exe" "D:/Cursor Project/RackViz_backup/"
   rm -rf "D:/Cursor Project/RackViz/src-tauri/target"
   ```

4. **从源头压缩（可选，需全量重编）**：在 `Cargo.toml` 追加 `[profile.release] strip = "symbols"`，可减少 `.pdb` 生成。效果需实测验证。

---

## 五、后续注意事项

### 5.1 🔴 已知 P0 缺陷：COALESCE 无法置空

**现象**：把设备从机柜拖回右侧「资源池」，提示成功，但刷新后设备仍在原机柜 U 位上。**"下架"功能实际是坏的。**

**根因**（`db/devices.rs:134-136`）：
```sql
UPDATE devices SET
    rack_id = COALESCE(?3, rack_id),   -- 传 NULL 时 → COALESCE(NULL, rack_id) = 原值！
    start_u = COALESCE(?4, start_u),
    end_u   = COALESCE(?5, end_u)
```

**受影响的 3 个前端调用点**：
1. `RackView.tsx:306-317` `handleStockDrop`（拖回资源池）
2. `DeviceDetailPanel.tsx:115`「未上架」按钮
3. `DeviceList.tsx:203-205` `handleDeviceOk`（状态选"未上架"）

**修复方向**：改用显式字段列表，或引入 sentinel（如 `-1` 表示置空），或在 Rust 侧判断 `Option` 为 `None` 时改写 `SET col = NULL`。
**注意**：`racks.room_id`、`devices.device_model_id` 同样受影响（共 28 处 COALESCE，波及 `db/racks.rs:70-72`、`db/rooms.rs:54`、`db/device_models.rs:63-64`）。

### 5.2 其他 P0 / P1 待办

| 编号 | 项 | 证据 | 级别 |
|------|-----|------|------|
| S-01 | CSP 配置 | `tauri.conf.json:27` `"csp": null` | 🔴 P0 |
| D-01 | 迁移事务保护 | `migration.rs` 全文无 `BEGIN`/`COMMIT` | 🔴 P0 |
| D-02 | 数据库索引（7 个） | 当前 **0 个索引** | 🔴 P0 |
| D-04 | COALESCE 置空 Bug | 见 5.1 | 🔴 P0 |
| S-05 | LIKE 通配符转义 | 无 `escape_like`；`db/devices.rs:38` | 🔴 P0 |
| P0-05 | 类型定义统一 | `types/index.ts` 与 `tauri-api.ts` 各一套，12 个重复接口 | 🔴 P0 |
| E-01/02 | 前端错误处理 | 6 处 `console.error`，~12 处 invoke 无 try/catch | 🔴 P0 |
| B-01 | 连接池错误分类 | `map_err(AppError::io)` **33 处** | 🟡 P1 |
| D-03 / D-05 | UNIQUE 约束 / find_or_create 竞态 | `migration.rs` 无 UNIQUE；`db/racks.rs:101-112` SELECT→INSERT | 🟡 P1 |
| B-02 / B-03 | `.unwrap()` **9 处** / `.ok()` 吞错误 **4 处** | | 🟡 P1 |
| B-04 / B-05 | 导出阻塞 UI / N+1 查询 | `exports.rs` 4 处 `blocking_save_file()`；`excel.rs:159` | 🟡 P1 |
| F-01~F-21 | 前端质量 | `RackView.tsx` 879 行 / `DeviceList.tsx` 577 行（目标 <300）；`Layout` Context **23 字段**（目标 <10）；`as T` 断言 **15 处** | 🟡 P1 |
| Q-03~07 | Clippy / ESLint / pre-commit **均不存在**；**4 处版本号不一致** | Cargo `0.2.0` / tauri.conf `0.2.0` / package.json `1.0.0` / Layout 徽标 `v1.1` | 🟡 P1 |

**测试覆盖**：仅 `db/` 层 23 个单元测试；**命令层、`excel.rs`、`report.rs`、前端全部零测试**。

### 5.3 开发"坑位"清单

| 坑 | 说明 |
|----|------|
| 别动 `dragDropEnabled: false` | `tauri.conf.json:23`，改成 true 会让 HTML5 拖拽失效，核心交互报废 |
| 改 CSP 前先处理 Google Fonts | `index.html:9` 从 CDN 加载字体，CSP 一开就被阻断；国内网络下本身也是体验隐患 |
| 别改已有 `migrate_v0_to_v1` | 老用户数据库已跑过，改了会不一致。要加新的 `migrate_vX_to_vY` |
| **加迁移要改四处** | 新函数 + `CURRENT_VERSION` + `run()` 的 match + 老分支串联 |
| `racks.view` 只有 `front`/`rear` | 前端 `RackView.tsx:127` 按 `r.view === view` 过滤，**默认只显示 `front`** |
| 新增机柜默认是 `front` | 所以**背面视图默认是空的，不是 Bug** |
| `start_u`/`end_u` 可能为 null | 前端多处用 `!` 非空断言，会算 NaN |
| 导入按列序号解析 | 用户 Excel 列顺序必须严格匹配，无表头识别 |
| 前端两套类型需同步 | 改 `types/index.ts` 记得同步 `tauri-api.ts` |
| `release` 模式隐藏控制台 | `main.rs` 的 `windows_subsystem = "windows"`；调试时看日志要到 `%LOCALAPPDATA%\com.rackviz.app\logs\` |
| `build-exe.bat` 不设 MSVC 环境 | 依赖 `%RUSTUP_HOME%` 探测，若 Rust linker 未配好会失败 |

### 5.4 常用命令

```bash
# 开发（前端热更新 + Rust 编译，首次 10-20 分钟）
cd src-tauri && cargo tauri dev

# 只跑前端（无后端，invoke 会失败）
cd frontend && npm run dev

# 前端类型检查
cd frontend && npx tsc --noEmit

# 构建前端 → frontend/dist
cd frontend && npm run build

# 编译 Rust + 产出裸 exe
cd src-tauri && cargo build --release
# 产物：src-tauri/target/release/rackviz.exe

# 完整打包（MSI + NSIS）
cd src-tauri && cargo tauri build
# 产物：src-tauri/target/release/bundle/{msi,nsis}/

# Rust 单元测试（db 层 23 个用例）
cd src-tauri && cargo test

# 一键打包
./build-exe.bat
```

**环境要求**：Node 22.22.2 / Rust stable + MSVC toolchain（`stable-x86_64-pc-windows-msvc`）/ Windows 10-11 x64。
> 注意：Git Bash 下调用 cargo 传 `/c/Users/...` 路径会失败（MSYS 路径转换），请用 `C:/Users/...` 正斜杠风格。

### 5.5 接手开发必读顺序

1. **§2.4 前后端契约**（本文档）—— 理解 `invoke` 怎么走
2. `frontend/src/tauri-api.ts`（268 行）—— 看完就知道全部后端能力
3. `src-tauri/src/lib.rs`（76 行）—— 看完就知道全部命令注册
4. `src-tauri/src/models.rs`（167 行）—— 看完就知道全部数据结构
5. `frontend/src/hooks/useApiList.ts`（79 行）—— 看完就知道前端数据怎么流转
6. `frontend/src/pages/RackView.tsx:38-94` —— `findAvailableSlot`，业务核心算法
7. **§5.1 的 COALESCE Bug** —— 当前最严重的功能缺陷，建议第一个修
8. `docs/RackViz-v1.2-升级方案.md` —— 想知道接下来做什么看这份

### 5.6 建议下一步（按投入产出比）

1. **修 COALESCE 置空 Bug** —— 1 个真 Bug，影响核心"下架"流程，半天可修
2. **统一版本号** —— 4 处，10 分钟
3. **接线 2 个已实现的导出**（机柜部署图 / 单机柜）—— 加 2 个按钮，1 小时
4. **清理无用依赖** —— `tauri-plugin-shell`、`tauri-plugin-fs`、`dirs`、dependencies 段的 `tauri-build`，30 分钟
5. **加数据库索引 + UNIQUE 约束**（迁移 v3）—— 半天
6. **配 ESLint + Clippy + pre-commit** —— 半天
7. 之后按 `docs/RackViz-v1.2-升级方案.md` 的 Phase 1→5 推进

---

## 六、本次分析的状态说明

| 项 | 状态 |
|----|------|
| 源码修改 | ❌ **未修改任何源码**（全程只读分析） |
| 文件删除 | ❌ **未删除任何文件**（清理方案仅给出建议与命令，**未执行**） |
| 新增文件 | ✅ 3 份：`docs/analysis-架构分析报告.md`、`docs/analysis-磁盘占用与清理方案.md`、`docs/HANDOFF-项目交接文档.md`（本文件） |
| 数据可靠性 | 所有大小数字均为本机实测；行号基于 2026-07-27 代码快照，变更请以实际文件为准 |

---

*本文档由 SoftwareCompany 团队协作生成：架构分析 + 磁盘专项分析 + 综合交接。*
