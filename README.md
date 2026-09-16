# RackViz — 机房设备可视化管理系统

**单机 Windows 桌面应用：用拖拽可视化替代 Excel 台账，管理「哪台设备在哪个机柜的哪个 U 位」。**

当前版本 **v2.0.0**（开发中）· Tauri 2 · React 19 · TypeScript 7 · Ant Design 6 · SQLite

---

## 核心功能

| 域 | 能力 |
|---|---|
| 🖥️ 机柜可视化 | 逐 U 网格渲染、正/背面视图、50–200% 缩放、U 位使用率预警（>85% 红警）、设备悬浮详情 |
| 🔄 拖拽上下架 | 资源池↔机柜↔机柜自由迁移、U 位冲突检测、多 U 设备按固有高度上架、下架确认 |
| 📋 设备台账 | 11 列可配置、列宽拖拽、中文排序、服务端分页、四字段搜索 + 关键词高亮 |
| 🗑️ 回收站 | 软删除 + 30 天恢复窗口、批量删除、撤销（Ctrl+Z）+ 全局快捷键 |
| 📥 导入导出 | Excel 15 列映射导入（三重查重、5000 行上限、导入向导）、台账/部署图/单机柜导出、HTML 报表 |
| 🏷️ 二维码标签 | 设备二维码 + 打印布局（7 字段多行键值契约，纠错级别 M，安静区 4 模块） |
| 💾 备份恢复 | 一键数据库备份 / 恢复 |
| 🌓 主题 | 暗 / 亮 / 护眼绿 / 夜间蓝四套预设，偏好持久化 |
| ⚙️ 设置 | 日志开关（运行时生效、按天轮转保留 7 天）、一键打开日志目录 |

设备类型支持 8 种：server / switch / router / storage / nas / security / loadbalancer / other。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2（Rust，`custom-protocol`） |
| 后端 | rusqlite 0.40（bundled）· r2d2 连接池 · calamine / rust_xlsxwriter（Excel）· askama（HTML 报表）· flexi_logger |
| 前端 | React 19.3 · TypeScript 7.0（strict）· Ant Design 6.6 · Vite 8.3 · react-router-dom 7 · qrcode.react 4 |
| 数据库 | SQLite（WAL 模式，schema v5：时间戳 + 软删除 + 部分唯一索引） |

## 快速开始

**环境要求**：Windows 10/11 x64 · Node 22+ · Rust stable（MSVC 工具链）· VS2019 BuildTools（含 Windows SDK）

```bat
:: 一键构建（MSVC 自定位 → npm ci → 前端构建 → cargo build --release）
build.bat

:: 一键检查（tsc + cargo check + cargo test，门禁）
check.bat

:: 完整安装包（MSI + NSIS）
cd src-tauri && cargo tauri build

:: 开发模式（前端热更新，首次编译 10-20 分钟）
cd src-tauri && cargo tauri dev

:: 只跑前端（无后端，invoke 会失败）
cd frontend && npm run dev

:: 前端类型检查 / 离线红线检查
cd frontend && npx tsc --noEmit
cd frontend && npm run lint:offline
```

## 目录结构

```
RackViz/
├── build.bat / check.bat      # 一键构建 / 门禁脚本
├── docs/                      # 项目文档（见下）
├── frontend/                  # React 前端
│   └── src/
│       ├── pages/             # RackView（机柜可视化）/ DeviceList（台账）
│       ├── components/        # rack/ device/ layout 组件
│       ├── hooks/             # useApiList / useRackView / useUndoStack ...
│       ├── contexts/          # View / Room / Theme / Undo
│       ├── types/index.ts     # ★ 领域类型单一来源
│       ├── tauri-api.ts       # ★ 34 个 invoke 封装（前后端契约）
│       └── themes/presets.ts  # 主题预设
└── src-tauri/
    └── src/
        ├── lib.rs             # ★ 34 个命令注册（41-76 行）
        ├── models.rs          # 数据结构（含三态 Patch<T>）
        ├── migration.rs       # 迁移链 v0→v5（with_migration_tx）
        ├── commands/          # IPC 层：devices/racks/rooms/device_models/exports/settings/maintenance
        ├── db/                # 纯 SQL 数据访问层
        └── excel.rs / report.rs / backup.rs
```

## 文档索引

| 文档 | 用途 |
|---|---|
| [docs/00-项目资料总览.md](docs/00-项目资料总览.md) | **单一事实来源**：事实基线、代码入口索引、设计约束（新人必读） |
| [docs/RackViz-v1.2-升级方案.md](docs/RackViz-v1.2-升级方案.md) | 功能路线图（N-01~N-21 定义；§八 v1.3 候选已裁剪，见 CHANGELOG） |
| [docs/RackViz-代码审查标准与流程.md](docs/RackViz-代码审查标准与流程.md) | **强制约束**：P0/P1/P2 审查清单、红线、自动化检查 |
| [docs/RackViz-v1.2-落地记录.md](docs/RackViz-v1.2-落地记录.md) | v1.2 增量实现实证（COALESCE/Patch\<T\> 等历史决策） |
| [docs/analysis-磁盘占用与清理方案.md](docs/analysis-磁盘占用与清理方案.md) | 磁盘专项方法论（数字有过期，target 实际约 3.1 GB） |
| [docs/WORKBUDDY-新建项目提示词.md](docs/WORKBUDDY-新建项目提示词.md) | AI 协作（WorkBuddy）项目初始化提示词 |
| [CHANGELOG.md](CHANGELOG.md) | 版本历史与决策记录 |

## 开发红线（摘要，全文见《代码审查标准与流程》）

- ❗ `tauri.conf.json` 的 `dragDropEnabled: false` **不可改**（否则拖拽上下架报废）
- 设备查询默认必须带 `deleted_at IS NULL`（v5 部分唯一索引 + 回收站语义）
- 加数据库迁移必须**四处同步**且走 `with_migration_tx`；禁改 v0→v5 已有分支
- 前端类型只改 `types/index.ts`，禁止第二套接口
- 生产代码禁用 `unwrap()` / `expect()` / `panic!` / `print_*`
- SQL 一律参数化；LIKE 必须走 `escape_like()`
- 每个 `invoke` 必须 try/catch + 用户可见反馈
- 禁删 lock 文件 / docs / 源码目录 / gen/schemas / icons

## 数据与日志

| 项 | 位置 |
|---|---|
| 数据库 | `%LOCALAPPDATA%\com.rackviz.app\rackviz.db` |
| 日志 | `%LOCALAPPDATA%\com.rackviz.app\logs\`（按天轮转，保留 7 天） |

> release 版控制台隐藏，排查问题看日志目录。

## 提交规范

Conventional Commits（`feat:` / `fix:` / `chore:` / `docs:` …）。pre-commit 门禁：`cargo check` + `clippy -D warnings` + `cargo test` + `tsc --noEmit` + `eslint` + 离线红线检查。
