# RackViz — 机房设备可视化管理系统

**单机 Windows 桌面应用：用拖拽可视化替代 Excel 台账，管理「哪台设备在哪个机柜的哪个 U 位」。**

当前版本 **v2.1.2** · Tauri 2 · React 19 · TypeScript 7 · Ant Design 6 · SQLite

![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue) ![License](https://img.shields.io/badge/license-MIT-green) ![Tests](https://img.shields.io/badge/tests-125%20passing-brightgreen)

---

## 核心功能

| 域 | 能力 |
|---|---|
| 🖥️ 机柜可视化 | 逐 U 网格渲染、正/背面视图、50–200% 缩放、U 位使用率预警（>85% 红警）、设备悬浮详情 |
| 🔄 拖拽上下架 | 资源池↔机柜↔机柜自由迁移、U 位冲突与重叠检测、多 U 设备按固有高度上架、下架确认 |
| 📋 设备台账 | 11 列可配置、列宽拖拽、中文排序、服务端分页、五字段搜索（名称/编号/IP/型号/负责人）+ 关键词高亮 + 结果直达机柜视图定位 |
| 🗑️ 回收站 | 软删除 + 30 天恢复窗口、批量删除、彻底删除（两次确认、不可恢复）、撤销（Ctrl+Z）+ 全局快捷键 |
| ✏️ 批量编辑 | 多选统一改 机柜 / U 位 / 状态；U 位三模式（保持原位 / 清空下架 / 自动排布）；Ctrl+Z 撤销 |
| ✅ 输入验证 | 前后端双校验：U 位区间/边界/重叠互斥、IP 格式、状态枚举；Excel 导入非法行自动跳过并警告 |
| 📥 导入导出 | Excel 15 列映射导入（三重查重、5000 行上限、导入向导）、台账/部署图/单机柜导出、HTML 报表 |
| 🏷️ 二维码标签 | 台账内标签列 + 批量打印（A4 3×8 排版，7 字段契约扫码即得） |
| 💾 备份恢复 | 自动备份（每日首启、滚动保留 7 份）+ 备份管理面板 + 启动自检 / 一键恢复 |
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

## 安装与运行

**绿色单文件，无需安装**：到 [Releases](../../releases) 下载 `RackViz-v2.1.2-x64.exe`，放到任意目录**双击即用**。

- 前端资源已嵌入 exe，仅依赖系统 WebView2（Windows 10/11 自带）
- 数据库与日志自动生成于 `%LOCALAPPDATA%\com.rackviz.app\`，不写程序目录
- 卸载 = 删除 exe + 删除上述数据目录

**SHA-256 校验**（可选，Release 附件提供 `RackViz-v2.1.2-x64.exe.sha256`）：

```bat
certutil -hashfile RackViz-v2.1.2-x64.exe SHA256
```

比对输出首行与 `.sha256` 文件内容一致即未被篡改。

### 从源码构建

**环境要求**：Windows 10/11 x64 · Node.js 22+ · Rust stable（MSVC 工具链）· Visual Studio 2019 BuildTools（含 Windows SDK）· WebView2 Runtime（Win10/11 一般已内置）

```bat
:: 获取代码
git clone <repo-url> RackViz
cd RackViz

:: 一键构建（MSVC 自定位 → npm ci → 前端构建 → cargo build --release）
:: 产物：src-tauri\target\release\rackviz.exe（绿色单文件）
build.bat

:: 一键门禁检查（tsc + cargo check + cargo test）
check.bat

:: 开发模式（前端热更新，首次编译 10-20 分钟）
cd src-tauri && cargo tauri dev
```

### 开发调试

```bat
:: 只跑前端（无后端，invoke 会失败——用于纯 UI 样式调整）
cd frontend && npm run dev

:: 前端类型检查 / 离线红线检查
cd frontend && npx tsc --noEmit
cd frontend && npm run lint:offline
```

## 使用方法

1. **建机房 → 建机柜**：先在机房页创建机房，再到机柜页添加机柜（高度 4–48U，默认 42U，可设排/列/正背面）
2. **录入设备**：三种方式任选
   - 台账页「添加设备」逐台录入
   - 「导入」向导上传 Excel（15 列模板见 [`test-data/`](test-data/)，含 100 条样例与边界用例；支持跳过/覆盖两种更新模式）
   - 机柜视图直接从资源池拖拽上架
3. **日常管理**：机柜视图拖拽迁移设备；U 位不足/超界/重叠会被即时拦截；删除进回收站，30 天内可恢复，误操作 Ctrl+Z 撤销
4. **标签打印**：设备台账勾选（单选/多选同一通道）→ 打印标签 → A4 批量排版（3×8=24 标签/页，左码右文带设备名/位置/资产号可读标识，浅灰裁切线）
5. **导出**：台账/机柜部署图/单机柜 Excel 与 HTML 报表（机房 U 位占用率图表）

## 配置说明

应用为单机绿色设计，无需账号与服务端。

| 项 | 位置 / 方式 |
|---|---|
| 数据库 | `%LOCALAPPDATA%\com.rackviz.app\rackviz.db`（SQLite WAL） |
| 日志 | `%LOCALAPPDATA%\com.rackviz.app\logs\`（按天轮转，保留 7 天；设置页可开关） |
| 主题 | 右上角切换四套预设，偏好本地持久化 |
| 台账列 | 列显隐 / 列宽拖拽自定义，自动记忆 |
| 自动备份 | `%LOCALAPPDATA%\com.rackviz.app\backups\`（每日首次启动自动备份，滚动保留最近 7 份） |
| 手动备份 / 恢复 | 设置页「备份数据库」（自选路径）/「从备份恢复」（自选文件，重启生效） |

## 数据安全与备份

- **自动备份（推荐依赖）**：应用每日首次启动自动生成一致性快照（`VACUUM INTO`，规避 WAL 拷贝丢事务），滚动保留最近 7 份；设置页「自动备份管理」可查看、恢复、删除。
- **恢复流程**：备份管理 → 选择可用备份（未通过完整性校验的标注「损坏」不可恢复）→ 确认覆盖 → 重启应用生效。恢复前会自动备份当前数据作为最后防线。
- **启动自检**：应用启动时执行 `PRAGMA integrity_check` 与 schema 版本核对，异常时弹窗引导从备份恢复。
- **迁移/换机**：停止应用后拷贝 `%LOCALAPPDATA%\com.rackviz.app\` 整个目录（或用「备份数据库」导出单文件，在新机「从备份恢复」）。

> release 版控制台隐藏，排查问题看日志目录。

## 目录结构

```
RackViz/
├── build.bat / check.bat      # 一键构建 / 门禁脚本
├── docs/                      # 项目文档（见下）
├── test-data/                 # 示例与边界测试数据（Excel 导入演示）
├── frontend/                  # React 前端
│   └── src/
│       ├── pages/             # RackView（机柜可视化）/ DeviceList（台账）
│       ├── components/        # rack/ device/ layout 组件
│       ├── hooks/             # useApiList / useRackView / useUndoStack ...
│       ├── contexts/          # View / Room / Theme / Undo
│       ├── types/index.ts     # 领域类型聚合（IPC 类型见 types/generated/，由 ts-rs 生成）
│       ├── tauri-api.ts       # ★ 39 个 invoke 封装（前后端契约）
│       └── themes/presets.ts  # 主题预设
└── src-tauri/
    └── src/
        ├── lib.rs             # ★ 39 个命令注册（41-76 行）
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

## 提交规范

Conventional Commits（`feat:` / `fix:` / `chore:` / `docs:` …）。pre-commit 门禁：`cargo check` + `clippy -D warnings` + `cargo test` + `tsc --noEmit` + 离线红线检查；CI（windows-latest）与门禁对齐。

## 许可证

本项目基于 [MIT License](LICENSE) 开源发布。第三方依赖（Tauri、React、Ant Design、rusqlite 等）各自遵循其原始许可证，详见各依赖仓库。
