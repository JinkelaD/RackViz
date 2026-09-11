# RackViz 项目全貌认知报告

> 生成日期：2026-07-27
> 分析范围：`D:\Cursor Project\RackViz` 全量源码（Rust 后端 22 文件 / 前端 18 文件 / 3 份产品文档）
> 目标读者：从未接触本项目的工程师（接手开发、清理 `src-tauri/target` 前请先读本文）
> 说明：本报告为**只读分析**产物，未修改任何项目文件。

---

## 〇、30 秒速读

| 项目 | 结论 |
|------|------|
| **这是什么** | 单机 Windows 桌面应用，机房/机柜/设备的可视化资产管理工具（"机房设备管理系统"），替代 Excel 台账 |
| **技术形态** | Tauri 2（Rust 后端 + 系统 WebView）+ React 18 前端 + 本地 SQLite |
| **代码规模** | Rust 2,183 行 / 前端 TS+TSX 2,728 行 / CSS 2,265 行 —— **小型项目，一个人可全盘掌握** |
| **架构分层** | 前端 `tauri-api.ts`（invoke 封装）→ Rust `commands/`（校验+日志）→ `db/`（纯 SQL）→ SQLite（r2d2 连接池 4 连接） |
| **数据模型** | 4 张业务表 + 1 张配置表：`rooms` → `racks` → `devices`，`device_models`（型号字典），`settings`（KV） |
| **对外接口** | 28 个 `#[tauri::command]`，前端 `tauri-api.ts` 一一对应封装 |
| **当前状态** | **功能上等于 v1.1 完成态；v1.2 升级方案几乎一行未落地** |
| **最大风险** | 存在 1 个已确认的 P0 数据正确性 Bug（COALESCE 无法置空，导致"下架"功能实际失效），详见 §9.2 |
| **版本混乱** | 4 处版本号互不一致：`0.2.0` / `0.2.0` / `1.0.0` / `v1.1` |

---

## 一、技术栈梳理

### 1.1 前端

来源：`frontend/package.json`、`frontend/vite.config.ts`、`frontend/tsconfig.json`、`frontend/index.html`

| 类别 | 选型 | 版本 | 备注 |
|------|------|------|------|
| 框架 | React | ^18.3.1 | 函数组件 + Hooks，无 Class 组件 |
| 语言 | TypeScript | ^5.6.3 | `strict: true`，但 `noUnusedLocals/Parameters` 均为 `false` |
| 构建 | Vite | ^6.4.0 | React 插件 `@vitejs/plugin-react` ^4.5.2 |
| UI 库 | Ant Design | ^5.22.0 | 用于 Table / Modal / Form / Select / Tag / Dropdown / Popover |
| 图表/可视化 | **无第三方库** | — | 机柜视图是手写 DOM + CSS 绝对定位（见 §7.4） |
| 路由 | react-router-dom | ^6.28.0 | `BrowserRouter`，仅 2 个页面 |
| Tauri 桥接 | @tauri-apps/api | ^2.11.0 | 只用 `invoke`（`core` 子模块） |
| Tauri 插件 | plugin-dialog | ^2.7.1 | 导入时前端自己弹文件选择框 |
| Tauri 插件 | plugin-fs | ^2.5.1 | ⚠️ **已声明但前端从未 import，实际未使用** |

**没有状态管理库**（无 Redux/Zustand/Recoil/SWR），只有 React 内置 Context + Hooks，详见 §7.3。

Vite 关键配置（`frontend/vite.config.ts`）：

| 配置项 | 值 | 含义 |
|--------|-----|------|
| `server.port` | 5173 | 与 `tauri.conf.json` 的 `devUrl` 对应 |
| `server.strictPort` | true | 端口被占用直接报错，不自动顺延 |
| `envPrefix` | `['VITE_', 'TAURI_']` | 允许读取 Tauri 注入的环境变量 |
| `build.target` | `['es2021','chrome105','safari13']` | WebView2 兼容基线 |
| `build.minify` | `!TAURI_DEBUG ? 'esbuild' : false` | debug 构建不压缩 |
| `manualChunks` | `vendor`(react 三件套) / `antd`(antd+icons) | 手工分包，减小单文件体积 |
| `chunkSizeWarningLimit` | 800 KB | 阈值放得很宽 |

`tsconfig.json`：`"paths": { "@/*": ["src/*"] }` 配了 `@/` 别名，**但全项目实际没有使用 `@/` 导入**，全是相对路径。`jsx: react-jsx`、`moduleResolution: bundler`、`noEmit: true`。

`index.html`：从 Google Fonts CDN 加载 `Space Grotesk` + `JetBrains Mono`。⚠️ **离线环境首次启动字体加载会阻塞/回退**，这在国内网络下是个体验隐患（字体在 `main.tsx` 的 `fontFamily` token 与 CSS 中被引用）。

### 1.2 Rust 后端

来源：`src-tauri/Cargo.toml`

| Crate | 版本 | 用途 | 关键性 |
|-------|------|------|--------|
| `tauri` | 2 | 桌面框架，`features=["custom-protocol"]` | 核心 |
| `tauri-build` | 2 | 构建期代码生成（**注意：同时出现在 `[dependencies]` 和 `[build-dependencies]`，前者是多余的**） | 构建 |
| `tauri-plugin-dialog` | 2 | 原生保存/打开对话框 | 导出/导入必需 |
| `tauri-plugin-fs` | 2 | 文件系统权限 | ⚠️ 已注册但代码未直接调用（导出走 `std::fs::write`） |
| `tauri-plugin-shell` | 2 | 执行外部命令 | ⚠️ **完全未使用**，属于无用依赖+权限面 |
| `rusqlite` | 0.32 | SQLite 绑定，`features=["bundled","chrono"]`（静态编译 SQLite，无需系统依赖） | 核心 |
| `r2d2` + `r2d2_sqlite` | 0.8 / 0.25 | 连接池，`max_size(4)` | 核心 |
| `calamine` | 0.25 | **读取** xlsx（`open_workbook`） | 导入必需 |
| `rust_xlsxwriter` | 0.79 | **写出** xlsx（`save_to_buffer`） | 导出必需 |
| `askama` | 0.12 | 编译期 HTML 模板引擎，渲染报表 | 报表必需 |
| `chrono` | 0.4 | 日期解析/格式化，`features=["serde"]` | 核心 |
| `log` + `flexi_logger` | 0.4 / 0.29 | 可运行时开关的日志，按天轮转保留 7 天 | 日志 |
| `serde` + `serde_json` | 1 | 序列化 | 核心 |
| `dirs` | 6 | 系统目录解析 | ⚠️ **代码中未使用**（实际用 `tauri::Manager::path()`） |

**Crate 类型**：`[lib] name = "rackviz_lib", crate-type = ["rlib"]`。`main.rs` 只有 5 行，调用 `rackviz_lib::run()`。

> ⚠️ 注意 `[lib] crate-type = ["rlib"]` **不包含 `cdylib`/`staticlib`**，这是 Tauri 2 移动端所需。当前只做桌面端（Windows MSI/NSIS），无影响。

### 1.3 Tauri 配置与打包

来源：`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`build-exe.bat`

```jsonc
{
  "productName": "RackViz",
  "version": "0.2.0",                    // ⚠️ 与前端 1.0.0 / UI 徽标 v1.1 不一致
  "identifier": "com.rackviz.app",
  "build": {
    "frontendDist": "../frontend/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "cd ../frontend && npm run dev",
    "beforeBuildCommand": "cd ../frontend && npm run build"
  },
  "app": {
    "windows": [{ "title": "RackViz - 机房设备管理系统", "width": 1400, "height": 900,
                  "minWidth": 1024, "minHeight": 680, "dragDropEnabled": false }],
    "security": { "csp": null }          // 🔴 P0：CSP 关闭（v1.2 S-01 待办）
  },
  "bundle": { "targets": ["msi", "nsis"], /* Windows WiX zh-CN + NSIS currentUser */ }
}
```

**两个关键设计点**：

1. `dragDropEnabled: false` —— 禁用 Tauri 默认的原生文件拖放拦截，**否则 HTML5 拖拽（设备上下架的核心交互）会被 WebView 抢走**。这是必须的，改动前请三思。
2. `csp: null` —— 安全策略关闭。v1.2 方案要求配成 `default-src 'self'`，但改它会导致 `index.html` 的 Google Fonts CDN 被阻断，**两件事要一起改**。

**权限清单**（`capabilities/default.json`）：`core:default`、`dialog:default`+`allow-save`+`allow-open`、`fs:default`+`read-text-file`+`write-text-file`、`shell:default`。其中 `shell` 权限实际用不到，属于可以收紧的权限面。

**打包脚本 `build-exe.bat`**（根目录，Windows 批处理）：
```
前置校验 → 探测 Rust MSVC toolchain → 检查 Node → npm install → npm run build → cargo build --release
产出：src-tauri\target\release\rackviz.exe
```
> ⚠️ 注意：脚本第 109 行是 `cargo build --release`（不是 `cargo tauri build`），**只产出裸 exe，不做 MSI/NSIS 安装包**。要出安装包需手动跑 `cargo tauri build`。
> 另外脚本对 Rust toolchain 的路径探测依赖 `%RUSTUP_HOME%`，且**没有设置 MSVC 的 `vcvarsall` 环境**，若 Rust 未配好 linker 会失败。

---

## 二、目录结构与代码规模

### 2.1 完整目录树

```
D:\Cursor Project\RackViz\
├── .gitignore                          # 已忽略 target/ node_modules/ dist/ *.db
├── build-exe.bat                        # Windows 一键打包脚本（3.2 KB）
│
├── docs/                                # 3 份产品与技术文档（共 85 KB）
│   ├── RackViz-v1.1-产品介绍.md           #   15.6 KB  产品定位/功能全景/架构/性能
│   ├── RackViz-v1.2-升级方案.md           #   45.8 KB  89 个审计发现的修复计划 + 19 个新功能
│   ├── RackViz-代码审查标准与流程.md        #   23.9 KB  审查清单/红线/自动化配置
│   └── analysis-架构分析报告.md            #   ← 本报告
│
├── frontend/
│   ├── index.html                       # 入口 HTML（含 Google Fonts CDN）
│   ├── package.json / package-lock.json
│   ├── vite.config.ts / tsconfig.json
│   ├── public/                          # ⚠️ 空目录
│   ├── dist/                            # 构建产物（已 gitignore）
│   ├── node_modules/                    # 172 MB（已 gitignore）
│   └── src/                             # ★ 前端源码，18 个文件
│       ├── main.tsx                     # React 入口 + AntD 主题 token
│       ├── App.tsx                      # 路由表（2 个页面）
│       ├── tauri-api.ts                 # ★★ 28 个 invoke 封装 + 类型（前后端契约核心）
│       ├── types/index.ts               # 领域类型（Device/Rack/Room/DeviceModel）
│       ├── constants/labels.ts          # 设备类型中文映射
│       ├── contexts/ThemeContext.tsx    # 暗/亮主题（localStorage 持久化）
│       ├── hooks/
│       │   ├── useApiList.ts            # ★★ 通用 CRUD Hook（乐观更新）
│       │   ├── useDevices.ts / useRacks.ts / useRooms.ts / useDeviceModels.ts
│       ├── components/
│       │   ├── Layout.tsx               # 顶栏 + 设置弹窗 + Outlet Context 下发
│       │   ├── RoomTabs.tsx             # 机房标签页（拖拽排序）
│       │   ├── StatusBar.tsx            # 底部状态栏（缩放/搜索/统计）
│       │   └── DeviceDetailPanel.tsx    # 设备详情侧边栏
│       ├── pages/
│       │   ├── RackView.tsx             # ★★ 机柜可视化主页面（879 行，最大文件）
│       │   └── DeviceList.tsx           # 设备台账表格页（577 行）
│       └── styles/global.css            # 2,265 行全局样式（体积最大的单文件）
│
└── src-tauri/
    ├── Cargo.toml / Cargo.lock
    ├── build.rs                         # 4 行，仅 tauri_build::build()
    ├── tauri.conf.json
    ├── capabilities/default.json        # Tauri 权限清单
    ├── icons/                           # 应用图标
    ├── gen/schemas/                     # Tauri 自动生成的 ACL schema（勿手改）
    ├── templates/report.html            # Askama 报表模板
    ├── target/                          # ⚠️ 1.6 GB 构建缓存（已 gitignore，可安全删除）
    └── src/                             # ★ Rust 源码，22 个文件
        ├── main.rs                      # 5 行入口
        ├── lib.rs                       # ★★ Tauri Builder + 28 个命令注册
        ├── models.rs                    # ★★ 13 个数据结构（实体/Create/Update）
        ├── error.rs                     # AppError 统一错误 + 6 个 From 转换
        ├── state.rs                     # r2d2 连接池 + with_transaction 包装器
        ├── migration.rs                 # 数据库迁移 v0→v1→v2
        ├── logging.rs                   # flexi_logger 初始化与运行时重配
        ├── excel.rs                     # ★★ Excel 导入(查重)/导出
        ├── report.rs                    # Askama 报表渲染
        ├── commands/                    # ★★ IPC 命令层（校验 + 操作日志）
        │   ├── mod.rs
        │   ├── rooms.rs / racks.rs / devices.rs / device_models.rs
        │   ├── exports.rs               # 4 导出 + 1 导入
        │   └── settings.rs              # 日志开关 3 命令
        └── db/                          # 数据访问层（纯 SQL）
            ├── mod.rs
            ├── rooms.rs / racks.rs / devices.rs / device_models.rs
            └── settings.rs
```

### 2.2 代码行数统计

**Rust 后端 —— 合计 2,183 行**

| 文件 | 行数 | 说明 |
|------|------|------|
| `src-tauri/src/db/devices.rs` | 296 | 含 70 行单元测试 |
| `src-tauri/src/excel.rs` | 323 | 最大 Rust 文件，导入查重逻辑在此 |
| `src-tauri/src/db/racks.rs` | 160 | 含 46 行测试 |
| `src-tauri/src/models.rs` | 167 | 纯数据结构 |
| `src-tauri/src/db/device_models.rs` | 136 | 含 36 行测试 |
| `src-tauri/src/db/rooms.rs` | 116 | 含 50 行测试 |
| `src-tauri/src/error.rs` | 110 | |
| `src-tauri/src/migration.rs` | 97 | |
| `src-tauri/src/logging.rs` | 96 | |
| `src-tauri/src/lib.rs` | 76 | 命令注册清单 |
| `src-tauri/src/report.rs` | 89 | |
| `src-tauri/src/state.rs` | 75 | |
| `commands/exports.rs` | 107 | |
| `commands/devices.rs` | 65 | |
| `commands/settings.rs` | 65 | |
| `commands/racks.rs` | 53 | |
| `commands/rooms.rs` | 53 | |
| `commands/device_models.rs` | 53 | |
| `main.rs` / `db/mod.rs` / `db/settings.rs` / `commands/mod.rs` | 5 / 8 / 27 / 6 | |

> 其中单元测试约 202 行，分布在 `db/` 的 4 个文件（**仅覆盖 db 层，命令层与 excel.rs 零测试**）。

**前端 —— 合计 4,993 行（TS/TSX 2,728 + CSS 2,265）**

| 文件 | 行数 | 说明 |
|------|------|------|
| `styles/global.css` | 2,265 | 纯手写 CSS，无 CSS Modules / Tailwind / CSS-in-JS |
| `pages/RackView.tsx` | 879 | 🔴 God Component（规范要求 <300） |
| `pages/DeviceList.tsx` | 577 | 🔴 同上 |
| `components/Layout.tsx` | 252 | 13 个 useState |
| `tauri-api.ts` | 268 | 前后端契约 |
| `components/RoomTabs.tsx` | 163 | |
| `components/DeviceDetailPanel.tsx` | 138 | |
| `main.tsx` | 109 | |
| `hooks/useApiList.ts` | 79 | |
| `components/StatusBar.tsx` | 87 | |
| `contexts/ThemeContext.tsx` | 45 | |
| `types/index.ts` | 49 | |
| `hooks/useRacks.ts / useRooms.ts` | 19 / 13 | |
| `hooks/useDeviceModels.ts / useDevices.ts` | 13 / 14 | |
| `App.tsx` / `constants/labels.ts` | 16 / 7 | |

**磁盘占用**：

| 目录 | 大小 | 可删除 |
|------|------|--------|
| `src-tauri/target` | **1.6 GB** | ✅ 可安全删除（构建缓存，`cargo build` 会重建，首次约 10–20 分钟） |
| `frontend/node_modules` | 172 MB | ✅ 可安全删除（`npm install` 恢复） |
| `frontend/dist` | 构建产物 | ✅ 可删除 |
| 数据库 `rackviz.db` | 运行时生成 | ⚠️ **删除会丢用户数据**，位置见 §4.6 |

---

## 三、Rust 后端：模块职责

### 3.1 模块职责总表

| 模块 | 职责 | 对外暴露 |
|------|------|----------|
| `main.rs` | 进程入口。`#![cfg_attr(not(debug_assertions), windows_subsystem="windows")]` —— **release 模式隐藏控制台窗口** | `fn main()` |
| `lib.rs` | 装配 Tauri：注册 3 个插件、`setup` 里初始化 DB 与日志、`invoke_handler` 注册 **28 个命令** | `pub fn run()` |
| `models.rs` | 13 个 serde 结构体：4 个实体 + 4 个 Create + 4 个 Update + `ImportResult`。全部 `#[serde(rename_all="snake_case")]` | 类型定义 |
| `error.rs` | `AppError { code, message, detail }` + `ErrorCode` 枚举（7 码）+ 6 个 `From` 转换（rusqlite / io / serde_json / XlsxError / r2d2 / calamine） | 类型 + 构造器 |
| `state.rs` | `DbState { pool, app_handle }`；`RackVizConnectionCustomizer` 保证每条连接都执行 PRAGMA；`with_transaction()` 事务包装器 | `DbState::new()` / `log_dir()` / `with_transaction()` |
| `migration.rs` | `PRAGMA user_version` 版本化迁移：`v0→v1`（建 4 表）→ `v1→v2`（建 settings 表）。**CURRENT_VERSION = 2** | `run(conn)` |
| `logging.rs` | flexi_logger 初始化 + 全局 `LOGGER_HANDLE`（Mutex）支持运行时切换。开=文件+stderr(Info)，关=仅 stderr(Warn) | `init()` / `reconfigure()` / `log_dir()` |
| `excel.rs` | 3 个导出 + 1 个导入。导入含**三重查重**与自动创建关联实体 | `export_*` / `import_devices_excel()` |
| `report.rs` | Askama 渲染 `templates/report.html`，统计开机/离线/未上架数量 | `render_report(conn)` |
| `commands/*.rs` | IPC 层：取连接 → 参数校验 → 调 db → `log::info!` 记录操作 | 28 个 `#[tauri::command]` |
| `db/*.rs` | 纯 SQL 数据访问，不含业务逻辑 | 各实体的 CRUD |

### 3.2 分层调用关系

```
前端 invoke()
   │  (JSON over IPC)
   ▼
commands/*.rs          ← 参数校验（仅 name 非空/长度）、操作日志 log::info!
   │
   ▼
db/*.rs                ← 纯 SQL，参数化查询，COALESCE 部分更新
   │
   ▼
rusqlite::Connection   ← 来自 r2d2 池（max 4），自动 PRAGMA
   │
   ▼
SQLite (WAL 模式)
```

**旁支**：`excel.rs` / `report.rs` 直接调 `db::` 层，不经过 `commands/`。

### 3.3 `#[tauri::command]` 完整清单（28 个）

统一约定：所有命令返回 `Result<T, AppError>`；`AppError` 序列化为 `{ code, message, detail? }`，`code` 为 camelCase 枚举。

#### 设备（5）— `commands/devices.rs`

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_devices` | `rack_id: Option<i32>`, `search: Option<String>` | `Vec<Device>` | `search` 是 `name LIKE %s%`；两者可为 null |
| `get_device` | `id: i32` | `Option<Device>` | ⚠️ 前端未调用 |
| `create_device` | `data: DeviceCreate` | `Device` | 校验 name 非空 + ≤100 字符 |
| `update_device` | `id: i32`, `data: DeviceUpdate` | `Option<Device>` | 🔴 COALESCE Bug，见 §9.2 |
| `delete_device` | `id: i32` | `bool` | 硬删除 |

#### 机柜（5）— `commands/racks.rs`

| 命令 | 参数 | 返回 |
|------|------|------|
| `list_racks` | — | `Vec<Rack>`（按 `sort_order, id`） |
| `get_rack` | `id: i32` | `Option<Rack>` ⚠️ 前端未调用 |
| `create_rack` | `data: RackCreate` | `Rack`（默认高 42U） |
| `update_rack` | `id`, `data: RackUpdate` | `Option<Rack>` |
| `delete_rack` | `id` | `bool`（先 `UPDATE devices SET rack_id=NULL`） |

#### 机房（5）— `commands/rooms.rs`

| 命令 | 参数 | 返回 |
|------|------|------|
| `list_rooms` | — | `Vec<Room>`（按 `sort_order, id`） |
| `get_room` | `id: i32` | `Option<Room>` ⚠️ 前端未调用 |
| `create_room` | `data: RoomCreate` | `Room` |
| `update_room` | `id`, `data: RoomUpdate` | `Option<Room>` |
| `delete_room` | `id` | `bool`（先 `UPDATE racks SET room_id=NULL`） |

#### 设备型号（5）— `commands/device_models.rs`

| 命令 | 参数 | 返回 |
|------|------|------|
| `list_device_models` | — | `Vec<DeviceModel>`（按 name） |
| `get_device_model` | `id: i32` | `Option<DeviceModel>` ⚠️ 前端未调用 |
| `create_device_model` | `data: DeviceModelCreate` | `DeviceModel` |
| `update_device_model` | `id`, `data: DeviceModelUpdate` | `Option<DeviceModel>` |
| `delete_device_model` | `id` | `bool`（先置空 devices 外键） |

#### 导入导出（5）— `commands/exports.rs`

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `export_racks_excel` | `app: AppHandle` | `String`（保存路径） | ⚠️ **前端未接线**，功能已实现但无入口 |
| `export_devices_data_excel` | `app: AppHandle` | `String` | 设备台账 15 列 |
| `export_single_rack_excel` | `app`, `rack_id: i32` | `String` | ⚠️ **前端未接线** |
| `export_report_html` | `app` | `String` | Askama 渲染 |
| `import_excel_from_path` | `path: String` | `ImportResult` | 前端先用 dialog 插件选文件，再传路径 |

> **导出流程（重要）**：4 个导出命令都在 Rust 侧调 `blocking_save_file()` 弹原生保存对话框，**阻塞当前线程**；生成的是 `Vec<u8>`，由 `std::fs::write` 落盘，避免二进制走 IPC。返回值只是"保存到了哪"的字符串。

#### 设置（3）— `commands/settings.rs`

| 命令 | 参数 | 返回 |
|------|------|------|
| `get_logging_config` | — | `LoggingConfig { enabled, log_dir }`（**camelCase**） |
| `set_logging_enabled` | `app`, `enabled: bool` | `LoggingConfig`（写库 + 运行时重配日志） |
| `open_log_dir` | — | `()`（Windows 用 `explorer` 打开，macOS `open`，Linux `xdg-open`） |

> ⚠️ `LoggingConfig` 是**唯一使用 `camelCase` 的响应结构**（`log_dir`），其余模型全用 `snake_case`。改前端时注意这个不一致。

### 3.4 数据模型

来源：`models.rs`（13 个结构体）、`migration.rs`（建表 SQL）

#### 实体关系

```
rooms (机房)                    device_models (型号字典)
  │ 1                                  │ 1
  │ room_id (ON DELETE SET NULL)       │ device_model_id (ON DELETE SET NULL)
  ▼ N                                  ▼ N
racks (机柜)  ──── 1:N ────►  devices (设备)
   height_u (默认42)            start_u / end_u  (U 位区间)
   row / col / view / sort_order
```

**全部外键都是 `ON DELETE SET NULL` 软级联** —— 删机房不会删机柜，删机柜不会删设备，设备只是解除关联。这是刻意设计（"设备不丢失"）。

#### 表结构

```sql
rooms(id, name NOT NULL, location DEFAULT '', sort_order DEFAULT 0)

racks(id, name NOT NULL, height_u DEFAULT 42, row DEFAULT 0, col DEFAULT 0,
      view DEFAULT 'front', sort_order DEFAULT 0,
      room_id → rooms(id) ON DELETE SET NULL)

device_models(id, name NOT NULL, manufacturer DEFAULT '',
              type DEFAULT 'server', height_u DEFAULT 1, power_watt DEFAULT 0)

devices(id, name NOT NULL,
        device_model_id → device_models(id) ON DELETE SET NULL,
        rack_id → racks(id) ON DELETE SET NULL,
        start_u, end_u,                       -- U 位区间，可 NULL（未上架）
        ip_addresses, serial_no, asset_no, department, owner, function,
        purchase_date DATE, warranty_expire DATE,
        status DEFAULT 'unconfigured',        -- online|offline|unconfigured
        power_watt DEFAULT 0)

settings(key PRIMARY KEY, value NOT NULL)     -- 目前只有 logging_enabled
```

#### 关键字段语义

| 字段 | 取值域 | 语义 |
|------|--------|------|
| `devices.status` | `online` / `offline` / `unconfigured` | 开机 / 离线 / 未上架（在资源池里） |
| `racks.view` | `front` / `rear` | 正面 / 背面视图，前端按此过滤 |
| `device_models.type` | `server`/`switch`/`router`/`storage`/`security` | 决定设备块配色与图标 |
| `start_u` / `end_u` | 1..height_u，可 NULL | **闭区间**，1U 设备 `start_u == end_u` |
| `sort_order` | int | 机房与机柜都靠它排序，前端拖拽排序就是批量改这个字段 |

#### 迁移机制

`migration.rs` 用 `PRAGMA user_version` 做版本控制，`run()` 按当前版本逐级升级：
```
version 0 → migrate_v0_to_v1() → migrate_v1_to_v2()
version 1 → migrate_v1_to_v2()
version 2 → 无操作（当前最新）
```
> 🔴 **迁移脚本没有事务保护**（`migrate_*` 内部是多个 `execute_batch`，无 BEGIN/COMMIT），中途失败会留下半截 schema。这是 v1.2 D-01 待办项。
> 新增迁移时要：① 写 `migrate_v2_to_v3()` ② 更新 `CURRENT_VERSION` ③ 在 `run()` 的 match 里补上 `2 => migrate_v2_to_v3(conn)?`。

### 3.5 错误处理约定

`AppError` 序列化后字段：`{ code, message, detail? }`（`detail` 为 None 时不序列化）。

| ErrorCode（camelCase） | 触发场景 | 构造器 |
|------------------------|----------|--------|
| `notFound` | 实体不存在 | `AppError::not_found("机柜")` → message="机柜 不存在" |
| `validationError` | 参数校验失败 / JSON 格式错误 | `AppError::validation(msg)` |
| `databaseError` | rusqlite 错误 / 连接池获取失败 | 自动 `From` 转换 |
| `ioError` | 文件读写 / Excel 操作 / 打开目录失败 | `AppError::io(msg)` |
| `cancelled` | 用户取消保存对话框 | `AppError::cancelled(msg)` |
| `conflict` / `unknown` | 已定义但**当前代码中未使用** | — |

> 前端目前**几乎不解析 `code`**，多数地方只是 `console.error`。这是 v1.2 E-01/E-02 待办。

### 3.6 数据库文件位置

`lib.rs:20-22` 决定：
```
{ app_local_data_dir() }/rackviz.db
```
Windows 上 `%LOCALAPPDATA%\com.rackviz.app\rackviz.db`（Tauri 2 用 identifier 作为子目录）。
日志在同级 `logs/` 目录，文件名 `rackviz.log`，按天轮转保留 7 份。

---

## 四、前端：模块职责

### 4.1 路由结构

来源：`main.tsx`、`App.tsx`

```
BrowserRouter                      main.tsx:101
└── ThemeProvider                  main.tsx:102   主题 Context
    └── AntdThemeWrapper           main.tsx:63    AntD ConfigProvider（暗/亮 token）
        └── App                    App.tsx
            └── Routes
                └── Route element={<Layout />}     ← 无 path，作为布局路由
                    ├── /racks    → RackView      机柜可视化（默认页）
                    ├── /devices  → DeviceList    设备台账
                    └── *         → <Navigate to="/racks" replace />
```

**只有 2 个页面**。Layout 通过 `<Outlet context={ctxValue} />` 向下传递共享状态，子页面用 `useOutletContext<ContextType>()` 接收。

### 4.2 状态管理方案

**没有引入任何状态管理库**，分层如下：

| 层次 | 机制 | 位置 |
|------|------|------|
| 服务端数据 | 4 个自定义 Hook，各自独立 `useApiList` | `hooks/use{Devices,Racks,Rooms,DeviceModels}.ts` |
| 跨页面 UI 状态 | React Router `Outlet context`（Layout 下发） | `components/Layout.tsx:140` |
| 主题 | React Context + localStorage | `contexts/ThemeContext.tsx` |

#### `useApiList` —— 全项目最重要的数据 Hook

`hooks/useApiList.ts`，79 行。签名：

```ts
useApiList<T extends { id: number }>(
  listFn:   () => Promise<T[]>,
  createFn: (data: Partial<T>) => Promise<T>,
  updateFn: (id: number, data: Partial<T>) => Promise<T | null>,
  deleteFn: (id: number) => Promise<boolean>,
  options?: { optimistic?: boolean }
): { items, loading, refresh, create, update, remove }
```

**实现要点**：
- 用 4 个 `useRef` 持有传入的 4 个函数（`listFnRef.current = listFn` 每次渲染刷新），使 `refresh` 的 `useCallback` 依赖数组能保持为 `[]`，**避免闭包过期 + 无限刷新**。
- `refresh()`：设 loading → 拉取 → `setItems` → 关 loading。错误仅 `console.error`。
- `update/remove`：非乐观模式 = `await 后端 → refresh()`；乐观模式 = **先本地改 `items`，再发请求，失败则回滚到 `prev` 并 `refresh()`**。
- **只有 `useDevices` 开了 `optimistic: true`**（`useDevices.ts:11`），其余三个是保守模式。

> 🔴 已知缺陷（`useApiList.ts:51`）：乐观更新用 `{ ...d, ...data } as T` 断言，且回滚用的 `prev = items` 是**闭包捕获的旧值**，并发操作时可能回滚到错误状态。v1.2 F-20/P1-20 待办。

**4 个 Hook 都是薄封装**（每个 13–19 行），把 `tauri-api` 的函数适配给 `useApiList`。`useRacks` 额外导出 `updateQuiet(id, data)` —— 直接调 API 但**不触发 refresh**，用于批量排序场景（避免 N 次刷新）。

### 4.3 组件职责表

| 组件 | 行数 | 职责 | 关键 state / props |
|------|------|------|--------------------|
| `Layout.tsx` | 252 | 顶栏（Logo/版本/主题切换/设置/导航）+ `<Outlet context>` 下发 23 字段 + 设置弹窗（日志开关） | 13 个 useState：`view/zoom/searchQuery/showAddRack/selectedRoomId/roomName/...` |
| `RoomTabs.tsx` | 163 | 底部机房标签页：切换过滤、双击改名、拖拽排序、`+ 新建机房` | `draggedRoomId`, `dragOverRoomId` |
| `StatusBar.tsx` | 87 | 底部工具栏：`+机柜`、缩放 50–200%、搜索框、统计（设备数/开机数/U 位使用率） | 纯 props，无 state |
| `DeviceDetailPanel.tsx` | 138 | 右侧设备详情抽屉：只读字段 + 状态三态切换按钮 + 删除 | props: `selectedDeviceInfo`, `onUpdate`, `onRemove` |
| `pages/RackView.tsx` | 879 | 🔴 **机柜可视化核心**：机柜网格、U 位、设备块、拖拽上下架、资源池、4 个 Modal | 14 个 useState |
| `pages/DeviceList.tsx` | 577 | 设备台账：AntD 可配置列表格（列显隐/列宽拖拽/排序/分页）+ 设备 CRUD Modal + 型号管理 Modal + 导入导出 | 13 个 useState |

### 4.4 机柜可视化是怎么画出来的（无图表库）

`RackView.tsx` 的渲染逻辑，全靠 **CSS 绝对定位 + 固定 26px/U**：

```tsx
// 机柜体：高度 = U 数 × 26px
<div className="rack-body" style={{ height: `${rack.height_u * 26}px` }}>   // :492

  {/* 网格层：从高到低渲染 U 位格子，u = height_u - i */}
  <div className="rack-grid">
    {Array.from({length: rack.height_u}, (_, i) => {
      const u = rack.height_u - i;                                          // :495
      return <div className="rack-u" onDragOver=... onDrop=... />            // :497-508
    })}
  </div>

  {/* 设备层：绝对定位，top 从机柜顶部往下算 */}
  <div className="rack-devices">
    {devices.map(device => {
      const deviceHeight = (device.end_u! - device.start_u! + 1) * 26;      // :517  ⚠️ 非空断言
      const topPosition  = (rack.height_u - device.end_u!) * 26;            // :518  ⚠️ 非空断言
      ...
    })}
  </div>
</div>
```

**U 位坐标系**：`u=1` 在**底部**，`u=height_u` 在**顶部**（这是真实机柜的惯例）。所以 `top = (height_u - end_u) * 26`。

缩放用 `transform: scale(zoom/100)` 作用在 `#rack-grid` 上（`RackView.tsx:424`）。

> ⚠️ `:517-518` 的 `device.end_u!` / `device.start_u!` 是**非空断言**，若设备 `start_u/end_u` 为 NULL 会算出 `NaN` 并渗透进 CSS（元素消失）。规范要求改 `?? 0`。

### 4.5 拖拽上下架算法

`RackView.tsx:38-94` 的 `findAvailableSlot()` —— 这是业务核心算法，值得单独理解：

```
输入：targetU(目标U位), deviceHeight(设备高度), rackHeight(机柜高度),
     existingDevices(已占用区间), excludeDeviceId(排除自己)
1. 把已有设备的 [start_u, end_u] 展开成 occupied: Set<number>
2. 从 targetU 向下扫，收集所有能放下连续 deviceHeight 的起点 s
3. 再从 targetU+1 向上扫，同样收集
4. 按 |s - targetU| 升序排序，距离相同时取**更靠上**的（b - a）
5. 返回最优 { startU, endU }；无解返回 null（前端提示"U 位不足"）
```

**调用链**：
- 拖入机柜：`handleDrop` (`:256`) → `findAvailableSlot` → `update(id, {rack_id, start_u, end_u, status?})`
  - 若设备原状态是 `unconfigured`，自动改成 `offline`（`:292`）
- 拖回资源池：`handleStockDrop` (`:306`) → `update(id, {rack_id: null, start_u: null, end_u: null})` 🔴 **因 COALESCE Bug 实际无效，见 §9.2**

**拖拽高度计算**（`:246-248`）：优先用设备自身的 `end_u - start_u + 1`；若无则用关联型号的 `height_u`；再无则 1U 并提示告警（`:271-273`）。

### 4.6 主题与样式

**双轨制主题**：
1. **CSS 变量**：`ThemeContext` 把 `data-theme="dark"|"light"` 写到 `<html>`；`global.css` 用 `[data-theme=...]` 选择器定义两套 token（`global.css:47` 暗色 / `:96` 亮色，`global.css:7` 是主题无关的共享 token）。
2. **AntD token**：`main.tsx` 的 `AntdThemeWrapper` 根据主题切换 `theme.darkAlgorithm/defaultAlgorithm` + 两套 colorToken（`:9` darkToken / `:27` lightToken），并单独定制 `Modal`/`Table`/`Button` 组件 token。

持久化 key：`rackviz-theme`（localStorage），默认 `'dark'`。

**`global.css` 结构**（2,265 行，纯手写，无预处理器）：
```
:1-46    共享 token（spacing/radius/font/typography）
:47-95   暗色主题 token（[data-theme=dark]）
:96-144  亮色主题 token
:145-185 Reset & Base + 自定义滚动条
:186-314 App Shell：#app / #header / .logo / .nav-tabs / .header-btn
:315-460 机房标签页 .room-tabs 全套
:461-478 布局容器 .rackview-wrapper / .rackview-main
:479-518 #main / #canvas-container / #rack-grid（缩放容器）
:520-... 机柜 .rack / .rack-header / .rack-body / .rack-u / .rack-devices / .device-block
:...     资源池 .sidebar-* / 状态栏 .statusbar-* / 模态框 .modal-* / 设置 .settings-*
         + 设备类型配色（.type-server/.type-switch/...）+ 状态指示（.online.pulse 等）
```

---

## 五、前后端交互契约

### 5.1 invoke 调用全表

来源：`frontend/src/tauri-api.ts`（268 行，28 个函数）

| 前端函数 | Rust 命令 | 前端传参 key | 返回 |
|----------|-----------|--------------|------|
| `listRooms()` | `list_rooms` | — | `RoomResp[]` |
| `getRoom(id)` | `get_room` | `{ id }` | `RoomResp \| null` ⚠️ 未被调用 |
| `createRoom(data)` | `create_room` | `{ data }` | `RoomResp` |
| `updateRoom(id, data)` | `update_room` | `{ id, data }` | `RoomResp \| null` |
| `deleteRoom(id)` | `delete_room` | `{ id }` | `boolean` |
| `listRacks()` | `list_racks` | — | `RackResp[]` |
| `getRack(id)` | `get_rack` | `{ id }` | ⚠️ 未被调用 |
| `createRack(data)` | `create_rack` | `{ data }` | `RackResp` |
| `updateRack(id, data)` | `update_rack` | `{ id, data }` | `RackResp \| null` |
| `deleteRack(id)` | `delete_rack` | `{ id }` | `boolean` |
| `listDevices(rackId?, search?)` | `list_devices` | `{ rackId, search }`（**camelCase → 自动转 snake**，`?? null`） | `DeviceResp[]` |
| `getDevice(id)` | `get_device` | `{ id }` | ⚠️ 未被调用 |
| `createDevice(data)` | `create_device` | `{ data }` | `DeviceResp` |
| `updateDevice(id, data)` | `update_device` | `{ id, data }` | `DeviceResp \| null` |
| `deleteDevice(id)` | `delete_device` | `{ id }` | `boolean` |
| `listDeviceModels()` | `list_device_models` | — | `ModelResp[]` |
| `getDeviceModel(id)` | `get_device_model` | `{ id }` | ⚠️ 未被调用 |
| `createDeviceModel(data)` | `create_device_model` | `{ data }` | `ModelResp` |
| `updateDeviceModel(id, data)` | `update_device_model` | `{ id, data }` | `ModelResp \| null` |
| `deleteDeviceModel(id)` | `delete_device_model` | `{ id }` | `boolean` |
| `exportRacksExcel()` | `export_racks_excel` | — | `string` ⚠️ 未被调用 |
| `exportDevicesDataExcel()` | `export_devices_data_excel` | — | `string` |
| `exportSingleRackExcel(rackId)` | `export_single_rack_excel` | `{ rackId }` | `string` ⚠️ 未被调用 |
| `exportReportHtml()` | `export_report_html` | — | `string` |
| `importExcelFromPath()` | `import_excel_from_path` | `{ path }` | `ImportResult` |
| `getLoggingConfig()` | `get_logging_config` | — | `{ enabled, logDir }` |
| `setLoggingEnabled(enabled)` | `set_logging_enabled` | `{ enabled }` | `{ enabled, logDir }` |
| `openLogDir()` | `open_log_dir` | — | `void` |

**命名约定（务必遵守）**：
- Rust 命令用 `snake_case`；Rust 参数用 `snake_case`。
- 前端传参对象用 **camelCase**（如 `rackId`），Tauri v2 会自动转换为 Rust 的 `rack_id`。
- **例外**：`data` 这个 key 本身就是整体对象，其**内部字段名保持 snake_case**（`device_model_id`、`start_u`…），因为它是整体反序列化为 Rust struct，走 `#[serde(rename_all="snake_case")]`。
- 响应体一律 `snake_case`，**唯一例外**是 `get_logging_config` 返回的 `logDir`（Rust 侧标了 `#[serde(rename_all="camelCase")]`）。

### 5.2 导入流程（唯一"前端先、后端后"的两段式调用）

```
用户点「导入」 DeviceList.tsx:393
   ↓
tauriApi.importExcelFromPath()          tauri-api.ts:231
   ├─ await import('@tauri-apps/plugin-dialog')    动态导入，拿到 open()
   ├─ await open({ filters: [{name:'Excel 文件', extensions:['xlsx','xls']}], multiple: false })
   ├─ 未选择 → 返回 { imported:0, skipped:0, errors:[] }（静默）
   ├─ 解析路径：selected 可能是 string 或 { path: string }     :243
   ↓
invoke('import_excel_from_path', { path })   → Rust excel.rs:209 import_devices_excel()
   ↓
返回 ImportResult { imported, skipped, errors }
   ↓
前端 message.success/warning/info 提示 + refresh()      DeviceList.tsx:164-180
```

> 与导出相反：导出的对话框由 **Rust 侧**弹（`blocking_save_file`），导入的对话框由 **前端**弹。这个不对称是历史遗留。

### 5.3 前端类型 vs Rust 模型

| 领域 | Rust (`models.rs`) | 前端 domain (`types/index.ts`) | 前端 API (`tauri-api.ts`) | 一致性 |
|------|--------------------|-------------------------------|---------------------------|--------|
| Device | `Device` | `Device` | `DeviceResp` | ✅ 字段一致；domain 的 `status` 是字面量联合类型，更严格 |
| Rack | `Rack` | `Rack` | `RackResp` | ✅ 一致（`view: 'front'\|'rear'` 更严格） |
| Room | `Room` | `Room` | `RoomResp` | ✅ 一致 |
| DeviceModel | `DeviceModel`（字段 `device_type` → 序列化为 `type`） | `DeviceModel`（字段 `type`） | `ModelResp` | ✅ 一致（Rust 用 `#[serde(rename="type")]`） |

🔴 **类型重复定义**：4 个实体各有 2 份 TS 定义（`types/index.ts` + `tauri-api.ts` 的 `*Resp`），加上 Create/Update 共约 **12 个重复接口**。这是 v1.2 F-08/P0-05 待办项。Hook 层还用 `as api.DeviceCreate` / `as Promise<Device>` 在两套类型间强转（共 11 处断言）。

### 5.4 交互流文字图

```
┌──────────────────────────────────────────────────────────────────────┐
│ 页面挂载                                                               │
│  Layout (useRooms/useRacks/useDevices/useDeviceModels)                 │
│    └─► useApiList ⇒ invoke('list_rooms' | 'list_racks' | ...)          │
│          └─► Rust: pool.get() → SELECT → Vec<T> → JSON                 │
│                └─► setItems(items) → Outlet context → 子页面渲染        │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│ 拖拽上架（RackView）                                                    │
│  设备块 onDragStart → setDraggingDevice(device)                        │
│    ↓  (每个 .rack-u 都有 onDragOver / onDrop)                          │
│  handleDragOver(rackId, u) → 计算设备高度 → setDropTarget 高亮预览       │
│    ↓
│  handleDrop(rackId, u) → findAvailableSlot() 求最优空位                 │
│    ↓  无解 → message.warning('U 位不足')
│    ↓  有解
│  update(id, {rack_id, start_u, end_u, status})   ← 乐观更新（本地先改）  │
│    └─► invoke('update_device') → with_transaction → UPDATE devices     │
│          └─► 成功 → refresh()；失败 → 回滚 + refresh()                  │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│ 导出 Excel（DeviceList 下拉菜单）                                       │
│  onClick → tauriApi.exportDevicesDataExcel()                          │
│    └─► invoke('export_devices_data_excel')                            │
│          ├─ Rust 查数据 → rust_xlsxwriter → Vec<u8>                    │
│          ├─ app.dialog().file().blocking_save_file()  ← 原生对话框      │
│          ├─ std::fs::write(path, data)                ← Rust 直接落盘   │
│          └─ return path (string)                                      │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│ 日志开关（Layout 设置弹窗）                                             │
│  打开弹窗 → getLoggingConfig() → 读 settings 表 logging_enabled        │
│  拨动开关 → setLoggingEnabled(checked)                                 │
│    └─► 写 settings 表 → logging::reconfigure(app, enabled)            │
│          └─► LOGGER_HANDLE.set_new_spec(Info | Warn)  ← 不用重启        │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 六、业务逻辑：这个应用到底解决什么问题

### 6.1 产品定位

来源：`docs/RackViz-v1.1-产品介绍.md`

> **RackViz 是一款面向数据中心运维人员的桌面级机房设备可视化管理系统。**

一句话概括：**用拖拽代替 Excel，管理机房里每台设备在哪个机柜的哪个 U 位。**

### 6.2 目标用户与痛点

**目标用户**：企业/机构数据中心的**运维工程师、IT 资产管理员**。单机单人使用（无多用户、无网络、无鉴权）。

| 痛点（文档原文） | RackViz 的解法 |
|------------------|----------------|
| 机柜设备位置靠 Excel 记录，不直观 | 可视化机柜网格，U 位精确到行 |
| 设备上下架需要改表改文档 | 拖拽即上架，一键即下架 |
| 多人运维缺少操作记录 | 操作审计日志（Rust 侧 `log::info!("[操作] ...")` + 可选文件日志） |
| 批量录入设备费时费力 | Excel 一键导入，三重查重 |
| 数据散落各处，报表靠手动 | Excel/HTML 报表一键生成 |

### 6.3 核心业务流程

```
1. 建机房（RoomTabs「+ 新建机房」）
2. 建机柜（StatusBar「+ 机柜」→ 选机房、设高度 4-48U）
3. 建型号（DeviceList「型号管理」→ 定义厂家/类型/高度U/功率）
4. 录入设备（手工「添加设备」或 Excel「导入」）
5. 上架（从右侧「资源池」拖设备到机柜 → 自动找最优 U 位）
6. 日常运维（改状态 开机/离线、迁机柜、下架回资源池）
7. 输出（导出设备台账 Excel / 机柜部署图 / 单机柜图 / HTML 报表）
```

### 6.4 关键业务规则（代码中硬编码的约束）

| 规则 | 位置 | 说明 |
|------|------|------|
| U 位坐标系：`u=1` 在底部 | `RackView.tsx:495` `const u = rack.height_u - i` | 符合真实机柜惯例 |
| 设备高度优先级 | `RackView.tsx:267-269` | 自身 `end_u-start_u+1` > 型号 `height_u` > 1U |
| 上架自动改状态 | `RackView.tsx:292` | `unconfigured` → 拖入后变 `offline` |
| U 位使用率预警阈值 85% | `RackView.tsx:439`、`StatusBar.tsx:82` | 超过显示 `warn` 红色 |
| 缩放范围 50%–200%，步长 10% | `Layout.tsx:60-62` | |
| 导入上限 5000 行 | `excel.rs:222` `max_rows` | 超出部分记入 `errors` |
| 导入三重查重 | `excel.rs:272-296` | ① 序列号唯一 ② 资产编号唯一 ③ 同机柜内名称唯一 |
| 导入自动建关联 | `excel.rs:237,246` | 型号/机柜不存在则 `find_or_create` |
| Excel 列顺序（15 列，固定） | `excel.rs:85-89` 表头 / `:232-270` 读取 | 名称/型号/机房/机柜/位置U/IP/序列号/资产编号/部门/责任人/功能/采购日期/质保到期/状态/功率 |
| 状态中文 ↔ 英文映射 | `excel.rs:264-268` | 开机↔online、离线↔offline、其余→unconfigured |
| 日期多格式容错 | `db/devices.rs:80-88` | 支持 `%Y-%m-%d` / `%Y/%m/%d` / `%Y.%m.%d` |
| 删除软级联 | `db/rooms.rs:61`、`racks.rs:84`、`device_models.rs:76` | 删除父级前先把子表外键置 NULL |

> ⚠️ **导入列顺序是硬编码的位置映射（第 0~14 列），没有表头识别**。用户 Excel 列顺序不对就会静默导入错误数据。且**第 3 列"机房"被读取后丢弃**（`excel.rs:242` `let _room_name = ...`），即导入不会自动关联机房 —— 这是 v1.2 N-05 待办项。

---

## 七、文档提取：产品规划与代码规范

### 7.1 `docs/RackViz-v1.1-产品介绍.md`（15.6 KB）

- **定位**：桌面级机房设备可视化管理系统，面向数据中心运维人员。
- **已完成功能**（v1.1）：机柜可视化网格、正/背面视图、50–200% 缩放、U 位使用率、拖拽上下架、U 位冲突检测、设备台账 11 列可配置、机房 CRUD + 拖拽排序、机柜 CRUD、型号管理、Excel 导入（15 列 + 三重查重）、3 种 Excel 导出、HTML 报表、Dark/Light 主题、日志收集开关。
- **技术架构**：与当前代码一致（React 18 / AntD 5.22 / Vite 6 / TS 5.6 / Tauri 2 / SQLite WAL / r2d2 4 连接）。
- **性能宣称**：安装包 18 MB、启动 <500ms、内存 30–50 MB、导入上限 5000 行。
- **版本历史**：v1.1 (2026-06-10) 从 **Python FastAPI + pywebview 完整迁移到 Rust + Tauri 2**。这解释了为什么代码里几乎没有历史包袱。
- ⚠️ 文档称"24 个命令""22 个 API 封装"，**实测是 28 / 28**，文档数字已过时。

### 7.2 `docs/RackViz-v1.2-升级方案.md`（45.8 KB，核心规划文档）

**依据**：`RackViz-v1.1-完整审计报告.md`（⚠️ **该报告文件不存在于 docs 目录**，无法交叉验证）。

**目标**：综合评分 5.4/10 → 8.0/10；P0 问题 13 → 0；P1 问题 55 → <10。预计 21 个工作日。

**Phase 划分**：

| Phase | 主题 | 工期 | 关键项 |
|-------|------|------|--------|
| Phase 0 | 准备 | 0.5d | 建分支、备份 DB、配 Clippy/ESLint/pre-commit、统一版本号 |
| Phase 1 | 安全与数据底线（P0 全清） | 3d | CSP、LIKE 转义、加索引、COALESCE 修复、类型统一、错误处理 |
| Phase 2 | 后端健壮性 | 3d | 连接池错误分类、UNIQUE 约束、find_or_create 竞态、导出异步化 |
| Phase 3 | 前端重构 | 4d | 拆 RackView/DeviceList、拆 Context、useMemo 优化、类型统一 |
| Phase 4 | 功能增强 | 8d | 分页、多字段搜索、导入更新模式、撤销、快捷键、时间戳、软删除 |
| Phase 5 | 质量基建 | 3d | 代码去重、自动化检查、CI |

**Phase 4 新功能清单（19 项，N-01 ~ N-19）**：

| 编号 | 功能 | 工期 |
|------|------|------|
| N-01 | 分页查询（`list_devices` 加 offset/limit） | 1d |
| N-02 | 多字段搜索（name/ip/serial_no/asset_no） | 0.5d |
| N-03 | 搜索结果高亮 | 0.5d |
| N-04 | 导入更新模式（跳过重复 / 覆盖更新） | 1d |
| N-05 | 导入关联机房（当前 `_room_name` 被丢弃） | 0.5d |
| N-06 | 导入进度显示 | 0.5d |
| N-07 | Excel 导出增强（冻结首行/自动筛选/条件格式） | 1d |
| N-08 | 时间戳字段（created_at/updated_at） | 0.5d |
| N-09 | 软删除（deleted_at，30 天可恢复） | 1d |
| N-10 | 输入验证增强（start_u ≤ end_u、IP 格式、枚举） | 0.5d |
| N-11 | **操作撤销 Ctrl+Z**（UndoStack） | 2d |
| N-12 | 键盘快捷键（Ctrl+N/F/Z、Delete、F5） | 1d |
| N-13 | 拖拽增强（tooltip + 确认提示） | 0.5d |
| N-14 | 设备二维码 + 打印标签 | 0.5d |
| N-15 | 多套主题预设 | 0.5d |
| N-16 | 设备拓扑图（d3-force / cytoscape） | 2d |
| N-17 | 报表图表化 | 1d |
| N-18 | 数据备份与恢复 | 1d |
| N-19 | 多语言（i18next） | 2d |

**验收标准要点**：组件 <300 行、useState <8 个、Context 字段 <10、生产路径 `.unwrap()` = 0、`as T` 断言 = 0、`any` = 0、版本号四处统一为 1.2.0。

**v1.3 候选**：多用户协同、数据库加密（SQLCipher）、PostgreSQL 网络化、设备监控、3D 可视化、网络自动发现。

### 7.3 `docs/RackViz-代码审查标准与流程.md`（23.9 KB）

**严重级别**：P0 阻断合并 / P1 建议修复 / P2 锦上添花。

**必须遵守的硬性规则（摘要）**：

*Rust 侧*：
- SQL 必须参数化，禁止 `format!` 拼 SQL 值
- LIKE 必须转义 `%` `_`
- 生产路径禁止 `.unwrap()`
- 日志禁止记录 `serial_no` / `asset_no` / `ip_addresses`，只记 `id`
- 禁止新增 `COALESCE(?N, col)` 更新（无法置空）
- 连接池获取必须用 `?`（已有 `From` trait），禁止 `map_err(AppError::io)`
- 禁止 `.ok()` 吞数据库错误
- 禁止 `blocking_save_file()` 阻塞 UI
- 批量操作必须在事务内

*前端侧*：
- 所有 `await invoke()` / `await api.*` 必须有 try/catch + 用户可见提示
- 禁止 `as T` 强制断言、`!` 非空断言、`any`
- 类型定义单一来源，禁止跨文件复制 ContextType
- 组件 >300 行必须拆分
- `useCallback` 依赖必须稳定（用 `useRef` 而非 `[items]`）

*数据库侧*：
- 新表必须有 `created_at`/`updated_at`
- WHERE/JOIN 字段必须有索引
- 唯一字段必须有 UNIQUE 约束
- 迁移脚本必须用事务
- `find_or_create` 必须用 `INSERT ... ON CONFLICT`

**审查流程**：提交者自查（10 分钟，有现成 rg 命令速查）→ PR → 审查者逐文件审（30–60 分钟）→ P0 退回 / P1 批准+建 Issue / P2 记录。

**建议的自动化配置**：Clippy（`unwrap_used`/`expect_used`/`panic` = warn）、ESLint（`no-explicit-any`/`no-non-null-assertion`/`react-hooks/exhaustive-deps`）、pre-commit hook（cargo check + clippy + test + tsc + eslint）。⚠️ **这三套配置目前一个都没落地**（无 `.clippy.toml`、无 `.eslintrc`、无 `.husky/`）。

### 7.4 三份文档的关系

```
v1.1 产品介绍 ──(审计后发现问题 89 个)──► v1.2 升级方案
                                              │
                                              ├─ 指导 ──► 代码审查标准与流程（把审计发现固化为检查清单）
                                              │
                                              └─ 执行 ──► 当前代码（几乎为零）
```

---

## 八、v1.2 规划 vs 当前代码：实现度对照

**结论：v1.2 升级方案基本没有落地。** 代码仍停留在 v1.1 完成态，且版本号四处不一致。

### 8.1 已实现（v1.1 功能全部在位）

✅ 机柜可视化网格与 U 位渲染　✅ 正/背面视图（`view` 字段过滤）　✅ 50–200% 缩放
✅ U 位使用率 + 85% 预警　✅ 拖拽上架 / 机柜间迁移 / 拖回资源池（**后者有 Bug**）
✅ 资源池（未分配设备）　✅ 设备台账表格（列显隐 / 列宽拖拽 / 排序 / 分页）
✅ 机房 CRUD + 拖拽排序 + 双击改名　✅ 机柜 CRUD + 左右移动
✅ 型号管理 CRUD + 搜索　✅ Excel 导入（三重查重 + 自动建关联）
✅ Excel 导出（台账 / 机柜部署图 / 单机柜）　✅ HTML 报表（Askama）
✅ Dark/Light 主题 + localStorage 持久化　✅ 日志收集开关 + 运行时切换 + 按天轮转 + 打开日志目录

### 8.2 未实现（按优先级）

| 编号 | 项 | 实测证据 | 级别 |
|------|-----|----------|------|
| S-01 | CSP 配置 | `tauri.conf.json:27` `"csp": null` | 🔴 P0 |
| D-01 | 迁移事务保护 | `migration.rs` 全文无 `BEGIN`/`COMMIT` | 🔴 P0 |
| D-02 | 数据库索引（7 个） | `migration.rs` 无 `CREATE INDEX` | 🔴 P0 |
| D-04 | **COALESCE 置空 Bug** | `db/devices.rs:134-136` / `racks.rs:70-72` / `rooms.rs:54` / `device_models.rs:63-64` 共 28 处 COALESCE | 🔴 P0 |
| S-05 | LIKE 通配符转义 | 无 `escape_like`；`db/devices.rs:38` `format!("%{}%", s)` | 🔴 P0 |
| P0-05 | 类型定义统一 | `types/index.ts` 与 `tauri-api.ts` 各一套，12 个重复接口 | 🔴 P0 |
| E-01/E-02 | 前端错误处理 | 实测 6 处 `console.error`，多数 `await update()` 无 try/catch | 🔴 P0 |
| B-01 | 连接池错误分类 | 实测 `map_err(\|e\| AppError::io)` **33 处**（文档称 26） | 🟡 P1 |
| D-03 | UNIQUE 约束 | `migration.rs` 无 `UNIQUE` | 🟡 P1 |
| D-05 | find_or_create 竞态 | `db/racks.rs:101-112`、`device_models.rs:87-98` 仍是 SELECT→INSERT | 🟡 P1 |
| B-03 | `.ok()` 吞错误 | 实测 4 处（文档称 7）：`db/devices.rs:86,201,210`、`db/settings.rs:9` | 🟡 P1 |
| B-02 | 生产路径 `.unwrap()` | 实测 **9 处**（`lib.rs:25`、`logging.rs:45,57,77,87`、`db/{devices,device_models,racks,rooms}.rs` 各 1） | 🟡 P1 |
| B-04 | 导出异步化 | `commands/exports.rs` 4 处 `blocking_save_file()` | 🟡 P1 |
| B-05 | N+1 查询 | `excel.rs:159` 在 `for rack in racks` 循环内调 `list_devices` | 🟡 P1 |
| F-01/F-03 | 组件拆分 | `RackView.tsx` 879 行、`DeviceList.tsx` 577 行（目标 <300） | 🟡 P1 |
| F-04 | LayoutContext 拆分 | `Layout.tsx:11-35` 的 `LayoutContext` 有 **23 个字段**（目标 <10） | 🟡 P1 |
| F-05~07 | useState 合并 | `RackView` 14 个、`Layout` 13 个、`DeviceList` 13 个（目标 <8） | 🟡 P1 |
| F-08/F-09 | `as T` 断言 | 实测 **15 处**（hooks 里 11 处 + DeviceList 4 处） | 🟡 P1 |
| F-10 | `any` 类型 | `useApiList.ts:4` `Record<string, any>`（1 处） | 🟡 P1 |
| F-12~19 | useMemo / useCallback 优化 | `RackView.tsx:132-137` 每次渲染重建 3 个查找函数；`DeviceList.tsx:295` `filteredDevices` 无 useMemo；`:306` 列定义在组件内 | 🟡 P1 |
| F-21 | ContextType 重复 | `RackView.tsx:12-36` 完整复制了 Layout 的 Context 接口 | 🟡 P1 |
| Q-03~05 | Clippy / ESLint / pre-commit | 三个配置文件**均不存在** | 🟡 P1 |
| Q-07 | 版本号统一 | Cargo `0.2.0` / tauri.conf `0.2.0` / package.json `1.0.0` / Layout 徽标 `v1.1` | 🟡 P1 |
| N-01 | 分页查询 | `list_devices` 无 offset/limit，前端分页是 AntD 客户端分页 | 🟢 新功能 |
| N-02/N-03 | 多字段搜索 / 高亮 | `list_devices` 只搜 `name`；前端只做 `name` 匹配 dim 效果 | 🟢 新功能 |
| N-04~N-07 | 导入导出增强 | 无更新模式、无进度、无冻结行；`_room_name` 被丢弃 | 🟢 新功能 |
| N-08/N-09 | 时间戳 / 软删除 | 无 `created_at`/`updated_at`/`deleted_at` 字段 | 🟢 新功能 |
| N-11/N-12 | 撤销 / 快捷键 | 代码中无 `UndoStack`、无键盘监听；`tauri-plugin-global-shortcut` 未安装 | 🟢 新功能 |
| N-16~N-19 | 拓扑图 / 图表 / 备份 / 多语言 | 全部无 | 🟢 新功能 |

### 8.3 实测质量指标 vs v1.2 目标

| 指标 | v1.2 基线（文档） | **实测** | v1.2 目标 | 差距 |
|------|------------------|----------|-----------|------|
| 生产路径 `.unwrap()` | 9 | **9** ✅ 与文档一致 | 0 | 需改 9 处 |
| 连接池 `map_err(AppError::io)` | 26 | **33** ❗ 比文档多 7 | 0 | 需改 33 处 |
| `.ok()` 吞错误 | 7 | **4** ❗ 比文档少 3 | 0 | 需改 4 处 |
| 前端 `as T` 断言 | ~8 | **15** ❗ | 0 | 需改 15 处 |
| 前端 `any` | 1 | **1** ✅ | 0 | 需改 1 处 |
| 最大组件行数 | 880 | **879** ✅ | <300 | 需拆 2 个文件 |
| 最大 useState 数 | 13 | **14**（RackView） | <8 | 需合并 |
| Context 字段数 | 23 | **23** ✅ | <10 | 需拆分 |
| 缺少索引的查询 | 7 | **0 个索引** ❗ | 0 | 需加 7 个索引 |
| 无 UNIQUE 约束字段 | 4 | **0 个约束** | 0 | 需加 4 个 |
| 无 try/catch 的 invoke | 21 | **~12** | 0 | 需补错误处理 |
| 版本号一致 | — | **4 处不一致** | 全为 1.2.0 | 需统一 |

---

## 九、关键代码位置索引（"想改 X 看哪里"）

### 9.1 后端（Rust）

| 想改什么 | 文件:行 | 函数 / 位置 |
|----------|---------|-------------|
| **注册新命令** | `lib.rs:44-73` | `tauri::generate_handler![...]` 数组，加一行 |
| 注册 Tauri 插件 | `lib.rs:16-18` | `.plugin(tauri_plugin_xxx::init())` |
| 改数据库路径 | `lib.rs:20-22` | `app_local_data_dir().join("rackviz.db")` |
| 加/改数据表 | `migration.rs:21` `migrate_v0_to_v1`、`:87` `migrate_v1_to_v2`、`:3` `CURRENT_VERSION`、`:10` match 分支 | 新增迁移需四处同步 |
| 加数据库索引 | `migration.rs` | 需新增 `migrate_v2_to_v3()`（当前无任何索引） |
| 改设备 CRUD SQL | `db/devices.rs:27/69/90/121/216` | `list_devices` / `get_device` / `insert_device` / `update_device` / `delete_device` |
| **改 U 位置空逻辑（P0 Bug）** | `db/devices.rs:134-136` | `rack_id = COALESCE(?3, rack_id)` ← 罪魁祸首 |
| 改机柜 CRUD SQL | `db/racks.rs:5/24/43/60/83` | list / get / insert / update / delete |
| 改机房 CRUD SQL | `db/rooms.rs:5/20/35/48/60` | 同上 |
| 改型号 CRUD SQL | `db/device_models.rs:5/22/39/54/75` | 同上 |
| 改设备搜索逻辑 | `db/devices.rs:27-67` | 动态拼 WHERE；`search` 只匹配 `name` |
| 加搜索字段（多字段搜索） | `db/devices.rs:35-40` | 在 conditions 里加 `OR` 分支 |
| 改导入查重规则 | `excel.rs:272-296` | 三重查重：序列号 / 资产编号 / 同机柜同名 |
| 改导入列映射 | `excel.rs:232-270` | 按**列序号**读取（0-14），改这里 |
| 让导入关联机房 | `excel.rs:242` | `let _room_name = ...` ← 读取后丢弃，改成 `find_or_create_room` |
| 改导出行数上限 | `excel.rs:222` | `let max_rows = 5000u32` |
| 改导出 Excel 表头 | `excel.rs:85-89`（台账）、`:143-150`（机柜图） | 15 列表头数组 |
| 改设备台账导出格式 | `excel.rs:71-132` | `export_devices_data_excel` |
| 改机柜部署图导出 | `excel.rs:134-173` | `export_racks_excel` ← **有 N+1 查询**在 `:159` |
| 改 HTML 报表内容 | `report.rs:28-89` + `templates/report.html` | Askama 模板，变量在 `ReportTemplate` struct |
| 改报表统计口径 | `report.rs:34-48` | online/offline/unconfigured 计数 |
| 改日志配置 | `logging.rs:17` `init`、`:64` `reconfigure`、`:36-40` 轮转策略 | 保留 7 天在 `:39` `Cleanup::KeepLogFiles(7)` |
| 改日志目录 | `logging.rs:10` `log_dir()` | `app_data_dir.join("logs")` |
| 改连接池大小 | `state.rs:34` | `.max_size(4)` |
| 改 SQLite PRAGMA | `state.rs:17-21` | WAL / foreign_keys=ON / busy_timeout=5000 |
| 改事务行为 | `state.rs:60-75` | `with_transaction()`：BEGIN IMMEDIATE / COMMIT / ROLLBACK |
| 加错误码 | `error.rs:6-16` `ErrorCode`、`:27-44` 构造器 | 加枚举变体 + 构造器方法 |
| 加错误类型转换 | `error.rs:52-110` | 已有 6 个 `From` impl |
| 改参数校验（设备） | `commands/devices.rs:25-30` | 只校验 name 非空 + ≤100 字符 |
| 改参数校验（机柜/机房/型号） | `commands/racks.rs:21-23`、`rooms.rs:21-23`、`device_models.rs:21-23` | 只校验 name 非空 |
| 改导出对话框行为 | `commands/exports.rs:24-29`（及其他 3 处） | `blocking_save_file()` ← 阻塞 UI，v1.2 要求改异步 |
| 改日志文件命名 | `logging.rs:28-32` | basename `rackviz`，suffix `log`，无时间戳 |

### 9.2 🔴 最该先修的 Bug：COALESCE 无法置空

**现象**：把设备从机柜拖回右侧「资源池」，提示成功，但刷新后设备仍在原机柜 U 位上。

**根因**：SQL 用 `COALESCE(?n, col)` 做部分更新，而"下架"要传 `null`：
```sql
-- db/devices.rs:134-136
UPDATE devices SET
    rack_id = COALESCE(?3, rack_id),   -- 传 NULL 时 → COALESCE(NULL, rack_id) = 原值！
    start_u = COALESCE(?4, start_u),   -- 同理
    end_u   = COALESCE(?5, end_u)      -- 同理
```

**受影响的 3 个前端调用点**：
1. `RackView.tsx:306-317` `handleStockDrop` — 拖回资源池
2. `DeviceDetailPanel.tsx:115` 「未上架」按钮
3. `DeviceList.tsx:203-205` `handleDeviceOk` — 状态选"未上架"时置空

**修复方向**（v1.2 D-04）：改用显式字段列表，或引入 sentinel（如 `-1` 表示置空），或在 Rust 侧判断 `Option` 为 `None` 时改写 `SET col = NULL`。**注意：`racks.room_id`、`devices.device_model_id` 同样受影响。**

### 9.3 前端

| 想改什么 | 文件:行 | 函数 / 位置 |
|----------|---------|-------------|
| **加新页面/路由** | `App.tsx:8-15` | 在 `<Route element={<Layout/>}>` 下加子路由 |
| 加导航标签 | `Layout.tsx:99-102` | `navItems` 数组 |
| 改 AntD 主题色 | `main.tsx:9-43` | `darkToken` / `lightToken`（两套都要改） |
| 改 AntD 组件级样式 | `main.tsx:76-89` | `components: { Modal, Table, Button }` |
| 改默认主题 | `contexts/ThemeContext.tsx:19` | `return 'dark'` |
| 改 localStorage key | `contexts/ThemeContext.tsx:12` | `rackviz-theme` |
| **加/改后端调用** | `tauri-api.ts` | 28 个函数，每个都是一行 `invoke()` |
| 改类型定义 | `types/index.ts`（domain）/ `tauri-api.ts:5-139`（API） | ⚠️ 两处需同步（v1.2 要求合并） |
| **改通用 CRUD 行为** | `hooks/useApiList.ts` | `refresh`/`create`/`update`/`remove` 全在这 |
| 改乐观更新策略 | `hooks/useApiList.ts:50-58`（update）、`:67-75`（remove） | 回滚逻辑有竞态 |
| 改某实体开不开乐观更新 | `useDevices.ts:11` `{ optimistic: true }` | 其余三个 Hook 无此参数 |
| 批量改机柜不触发刷新 | `hooks/useRacks.ts:14-16` | `updateQuiet()` |
| **改机柜渲染** | `RackView.tsx:436-591` | 机柜 map：header / body / grid / devices / footer |
| 改 U 位格子 | `RackView.tsx:494-512` | `Array.from({length: height_u})`，`u = height_u - i` |
| 改设备块定位 | `RackView.tsx:517-518` | `deviceHeight`/`topPosition` = `(end_u - start_u + 1) * 26` |
| **改 U 位分配算法** | `RackView.tsx:38-94` | `findAvailableSlot()` 核心算法 |
| 改拖拽上架 | `RackView.tsx:256-296` | `handleDrop` |
| 改拖回资源池 | `RackView.tsx:306-317` | `handleStockDrop` 🔴 受 COALESCE Bug 影响 |
| 改拖拽预览高亮 | `RackView.tsx:243-250` | `handleDragOver` → `setDropTarget` |
| 改设备高度推断 | `RackView.tsx:266-273` | 自身区间 > 型号 height_u > 1U |
| 改资源池面板 | `RackView.tsx:596-634` | 未分配设备列表 |
| 改缩放范围/步长 | `Layout.tsx:60-62` | 50–200%，步长 10% |
| 改 U 位预警阈值 | `RackView.tsx:439`、`StatusBar.tsx:82` | `> 85` 变红 |
| 改设备类型配色/图标 | `constants/labels.ts`（中文名）+ `global.css` 的 `.type-*` | 5 种类型 |
| 改设备状态文案 | `RackView.tsx:411-415`、`DeviceDetailPanel.tsx:5-9`、`DeviceList.tsx:287-291` | ⚠️ 3 处重复（v1.2 F-22 要求提取） |
| 改台账列定义 | `DeviceList.tsx:306-335` | `allDeviceColumns` 11 列 |
| 改可显示列清单 | `DeviceList.tsx:91-103` | `ALL_COLUMNS` |
| 改默认列宽 | `DeviceList.tsx:28-40` | `DEFAULT_COLUMN_WIDTHS` |
| 改列宽拖拽实现 | `DeviceList.tsx:42-89` | `ResizableTitle` 组件（纯手写 mousemove） |
| 改导入按钮逻辑 | `DeviceList.tsx:161-181` | `handleDeviceImport` |
| 改导出菜单 | `DeviceList.tsx:396-410` | Dropdown：Excel / HTML 两项 |
| 改设备表单字段 | `DeviceList.tsx:457-516` | 15 个 `Form.Item` |
| 改型号管理弹窗 | `DeviceList.tsx:519-574` | 表格 + inline 表单 |
| 改设备编辑弹窗（RackView 双击） | `RackView.tsx:732-819` | 8 个字段的表单 Modal |
| 改机柜编辑弹窗 | `RackView.tsx:821-875` | 名称 + 所属机房 |
| 改新建机柜弹窗 | `RackView.tsx:680-730` | 名称 + 高度(4-48U) + 机房 |
| 改机房标签页 | `components/RoomTabs.tsx:105-162` | 渲染 / 新建 / 拖拽排序 |
| 改机房拖拽排序算法 | `RoomTabs.tsx:62-101` | `handleDrop` 重排 `sort_order` 并批量 update |
| 改状态栏统计 | `StatusBar.tsx:39-83` | 设备数 / 开机数 / U 位使用率 |
| 改设备详情面板 | `components/DeviceDetailPanel.tsx` | 只读字段 + 状态切换 + 删除 |
| 改设置弹窗（日志） | `Layout.tsx:204-249` | 日志开关 + 打开目录按钮 |
| 改全局样式 | `styles/global.css` | token 在 `:7-144`，组件样式从 `:186` 起 |

### 9.4 构建与配置

| 想改什么 | 文件:行 | 说明 |
|----------|---------|------|
| 改窗口尺寸/标题 | `tauri.conf.json:13-24` | 1400×900，最小 1024×680 |
| 改打包目标（MSI/NSIS） | `tauri.conf.json:30-48` | `targets: ["msi","nsis"]` |
| 改应用图标 | `tauri.conf.json:33-38` + `src-tauri/icons/` | |
| **改 CSP**（v1.2 S-01） | `tauri.conf.json:27` | ⚠️ 改前先处理 `index.html:9` 的 Google Fonts CDN |
| 改 Tauri 权限 | `src-tauri/capabilities/default.json` | 建议收紧 `shell:default` |
| 改前端依赖 | `frontend/package.json` | |
| 改 Vite 端口 | `vite.config.ts:8` | 需与 `tauri.conf.json:8` `devUrl` 一致 |
| 改代码分包 | `vite.config.ts:18-21` | `manualChunks` |
| 改 Rust 依赖 | `src-tauri/Cargo.toml` | 可清理：`tauri-build`(dependencies 段)、`tauri-plugin-fs`、`tauri-plugin-shell`、`dirs` |
| 改打包脚本 | `build-exe.bat:109` | `cargo build --release` → 若要安装包改成 `cargo tauri build` |

---

## 十、新工程师上手指南

### 10.1 环境准备

| 依赖 | 要求 | 备注 |
|------|------|------|
| Node.js | 22.22.2（项目指定路径 `C:\Users\Jinkela\.workbuddy\binaries\node\versions\22.22.2\node.exe`） | 任意 Node 18+ 应该都行 |
| Rust | stable + **MSVC toolchain**（`stable-x86_64-pc-windows-msvc`） | 需 Visual Studio Build Tools 的 C++ 工具链 |
| 系统 | Windows 10/11 x64 | 代码有 macOS/Linux 分支但未测试 |

### 10.2 常用命令

```bash
# === 开发（前端热更新 + Rust 编译，首次 10-20 分钟）===
cd src-tauri && cargo tauri dev
# 等价于：先起 vite(5173)，再编译 Rust 并开窗口

# === 只跑前端（无 Rust 后端，invoke 会失败）===
cd frontend && npm run dev

# === 只跑前端类型检查 ===
cd frontend && npx tsc --noEmit

# === 构建前端 ===
cd frontend && npm run build       # = tsc -b && vite build → frontend/dist

# === 编译 Rust + 产出裸 exe ===
cd src-tauri && cargo build --release
# 产物：src-tauri/target/release/rackviz.exe

# === 完整打包（MSI + NSIS 安装包）===
cd src-tauri && cargo tauri build
# 产物：src-tauri/target/release/bundle/{msi,nsis}/

# === Rust 单元测试（覆盖 db 层，23 个用例）===
cd src-tauri && cargo test

# === 一键打包（Windows 批处理）===
./build-exe.bat                    # 在资源管理器双击即可
```

### 10.3 磁盘清理安全清单

| 路径 | 大小 | 能否删 | 恢复成本 |
|------|------|--------|----------|
| `src-tauri/target/` | **1.6 GB** | ✅ **完全安全** | 重新 `cargo build` 约 10–20 分钟 |
| `src-tauri/target/release/bundle/` | — | ✅ 删除后丢失已打好的安装包 | 重新 `cargo tauri build` |
| `frontend/node_modules/` | 172 MB | ✅ 安全 | `npm install` |
| `frontend/dist/` | 小 | ✅ 安全 | `npm run build` |
| `frontend/*.tsbuildinfo` | 小 | ✅ 安全 | 自动生成 |
| `src-tauri/gen/schemas/` | 小 | ⚠️ **不要删** | Tauri 自动生成，但删除可能导致构建异常；`cargo build` 会重新生成 |
| **数据库 `rackviz.db`** | — | 🔴 **严禁删除** | 位置：`%LOCALAPPDATA%\com.rackviz.app\rackviz.db`，删了用户数据全没 |
| **日志 `logs/`** | — | ✅ 可删 | 下次启动自动重建 |

> **推荐清理命令**（保留源码完整性）：
> ```bash
> rm -rf src-tauri/target        # 释放 1.6 GB
> # 需要时 cargo build 会完整重建
> ```
> `.gitignore` 已正确忽略 `src-tauri/target/`，所以删除不影响版本控制。
> ⚠️ 注意：本项目根目录**不是 Git 仓库**（实测 `git log` 无输出），所以没有版本控制兜底，删之前确认无需回滚。

### 10.4 接手开发前必读顺序

1. **本报告 §5（前后端交互契约）** — 理解 `invoke` 怎么走
2. `frontend/src/tauri-api.ts` — 268 行，看完就知道全部后端能力
3. `src-tauri/src/lib.rs` — 76 行，看完就知道全部命令注册
4. `src-tauri/src/models.rs` — 167 行，看完就知道全部数据结构
5. `frontend/src/hooks/useApiList.ts` — 79 行，看完就知道前端数据怎么流转
6. `frontend/src/pages/RackView.tsx:38-94` — `findAvailableSlot`，业务核心算法
7. **§9.2 的 COALESCE Bug** — 这是当前最严重的功能缺陷，建议第一个修
8. `docs/RackViz-v1.2-升级方案.md` — 想知道接下来做什么看这份

### 10.5 开发时的"坑位"提醒

| 坑 | 说明 |
|----|------|
| 别动 `dragDropEnabled: false` | `tauri.conf.json:23`，改成 true 会让 HTML5 拖拽失效，核心交互报废 |
| 改 CSP 前先处理 Google Fonts | `index.html:9` 从 CDN 加载字体，CSP 一开就被阻断 |
| 别在 `migration.rs` 改已有 `migrate_v0_to_v1` | 老用户数据库已跑过，改了会不一致。要加新的 `migrate_vX_to_vY` |
| 加迁移要改四处 | 新函数 + `CURRENT_VERSION` + `run()` 的 match + 老分支串联 |
| `racks.view` 只有 `front`/`rear` | 前端 `RackView.tsx:127` 按 `r.view === view` 过滤，默认只显示 `front` |
| 新增机柜默认是 `front` | 所以背面视图默认是空的，不是 Bug |
| `start_u`/`end_u` 可能为 null | 前端多处用了 `!` 非空断言，会算 NaN |
| 导出对话框阻塞 UI | `blocking_save_file()` 在大文件导出时会卡住界面 |
| 导入按列序号解析 | 用户 Excel 列顺序必须严格匹配，无表头识别 |
| 前端两套类型需同步 | 改 `types/index.ts` 记得同步 `tauri-api.ts` |
| `export_racks_excel` / `export_single_rack_excel` 无 UI 入口 | 后端已实现，前端没接线，加上按钮即可启用 |
| `get_device`/`get_rack`/`get_room`/`get_device_model` 无调用方 | 命令已注册但前端未用，属于预留接口 |

---

## 十一、总结

### 11.1 项目健康度评估

| 维度 | 评价 |
|------|------|
| **架构清晰度** | ⭐⭐⭐⭐⭐ 分层干净（commands / db / models / error / state），新人 1 天可看懂全貌 |
| **代码规模** | ⭐⭐⭐⭐⭐ 小（Rust 2.2k + TS 2.7k 行），无历史包袱（刚从 Python 重写过） |
| **功能完整度** | ⭐⭐⭐⭐ v1.1 承诺的功能基本都实现了，2 个导出功能缺 UI 入口 |
| **正确性** | ⭐⭐ 🔴 有 1 个 P0 数据 Bug（下架失效），无索引、无唯一约束 |
| **安全性** | ⭐⭐ CSP 关闭、LIKE 未转义、日志可能含敏感信息、权限面偏大 |
| **可维护性** | ⭐⭐⭐ 2 个 God Component（879/577 行）、23 字段 Context、类型双份定义、无 lint |
| **测试覆盖** | ⭐⭐ 仅 db 层 23 个单元测试，命令层与 excel/report 层零测试，前端零测试 |

### 11.2 建议的下一步（按投入产出比排序）

1. **修 COALESCE 置空 Bug**（§9.2）—— 1 个真 Bug，影响核心"下架"流程，半天可修
2. **统一版本号** —— 4 处，10 分钟
3. **接线 2 个已实现的导出功能**（机柜部署图 / 单机柜）—— 加 2 个按钮，1 小时
4. **清理无用依赖** —— `tauri-plugin-shell`、`tauri-plugin-fs`、`dirs`、Cargo dependencies 段的 `tauri-build`，30 分钟
5. **加数据库索引 + UNIQUE 约束**（迁移 v3）—— 半天，为后续性能打底
6. **配 ESLint + Clippy + pre-commit** —— 半天，防止质量继续下滑
7. 之后按 `docs/RackViz-v1.2-升级方案.md` 的 Phase 1→5 推进

---

*本报告由源码静态分析生成，所有行号基于 2026-07-27 的代码快照。代码变更后请以实际文件为准。*
