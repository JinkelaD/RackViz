# Changelog

RackViz 版本历史。格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，遵循语义化版本。

## [Unreleased] — v2.1.0 开发中

### 二维码标签打印重构（主理人 2026-09-17 拍板）

- **入口迁移**：移除「机柜管理 → 设备详情」的二维码标签展示（exe 界面内显示二维码无实际意义）；功能整体迁移至**设备台账**，经表格勾选（单选/多选同一通道）触达。
- **打印统一为批量模式**：取消原单码 185mm 整页打印流程；新批量打印为 A4 纵向 3×8=24 标签/页（63×34mm、裁切虚线），仅勾选 1 台时网格左上角输出唯一标签。
- **可区分性**：标签采用左码右文——二维码沿用 7 字段中文明文契约（`encodeDeviceQr` 不变），右侧新增可读摘要（设备名 / 机柜-U位 / 资产编号 / SN），解决打印后外观相似易贴错的问题。
- **贴签顺序**：机房 sort_order → 机柜 row/col → U 位升序，未上架设备排最后并显示「未上架」警示。
- **实现**：新增 `BatchPrintModal`（预览+统计+未上架警告）与 `PrintLabelCell`；打印输出经 body 级 portal，`@media print` 下隐藏应用外壳与 antd 浮层后仅保留 `.batch-print-root`；删除 `DeviceQrLabel` 组件与单码打印 CSS。
- **验证**：Playwright 打印媒体模拟——3 台混合选区（排序/警告/1 页）、单选（24 格仅左上角 1 标签）、sheet 高 277mm、二维码 22mm 矢量（≥0.4mm/模块可扫下限的十余倍）全部通过；`tsc --noEmit` 全绿。

## [2.0.0] - 2026-09-16（当前开发版本，安装包未发布）

### 版本决策

- 定版 **2.0.0**：依赖栈整体跃迁（React 18→19、TS 5→7、AntD 5→6、Vite 6→8）+ 接口面扩至 34 个 Tauri 命令，语义上够 major；呼应 Stage 1 提交 `chore(v2.0.0)!` 标记（主理人 2026-09-16 拍板）。
- **交付形态：绿色单文件 exe**（主理人 2026-09-16 拍板）：`bundle.targets: none`，不做 MSI/NSIS 安装包；`rackviz.exe` 自包含（前端资源嵌入、WebView2 系统自带、数据在 `%LOCALAPPDATA%`），双击即用。

### 产品决策（主理人 2026-09-16 拍板）

- **N-16 设备拓扑图：关闭**（产品不需要，不引入图库依赖）
- **N-19 多语言 i18n：关闭**（无多语种用户）
- **v1.3 路线调整**：砍掉多用户协同、网络化 PostgreSQL；**专心本地化深化**
- **SQLCipher 数据库加密：近期不纳入**（C 依赖编译代价与老库迁移成本，暂不排期）
- **N-10 输入验证增强：2026-09-16 完成 → Phase 4 达成 21/21 收官**

### 依赖跃迁（Stage 1 `0c77fb4`）

| 端 | 变更 |
|---|---|
| 前端 | React 19.3.0（原 18.3.1）、TypeScript 7.0.2（原 5.6.3）、Vite 8.3.0（原 6.4.0）、Ant Design 6.6.3（原 5.22.0）、react-router-dom 7.18.3（原 6.28.0） |
| 后端 | rusqlite 0.40、r2d2_sqlite 0.35、calamine 0.36、rust_xlsxwriter 0.99、askama 0.16 |
| 工程 | 新增 `[profile.release]`（opt-level="s" + lto="thin" + strip="symbols"）；Clippy lints（unwrap_used/expect_used/panic/print_* = warn） |

### 新功能（Phase 4 功能增强 21/21 收官）

- N-01 服务端分页、N-02 四字段搜索、N-03 搜索结果高亮
- N-04 导入更新模式（跳过/覆盖）、N-05 导入关联机房、N-06 导入进度显示、N-07 Excel 导出增强
- N-08 全表时间戳（created_at/updated_at）
- N-09 软删除 + 回收站 + 30 天恢复窗口
- N-10 输入验证增强（`0217b6f` 前端 / `f51ca2c` 后端，2026-09-16）：**前后端双校验**——
  U 位区间（end≥start 且 ≥1）、U 位边界（≤ 所选机柜 height_u）、**同机柜 U 位重叠互斥**
  （排除自身/软删，Conflict）、IP 列表格式、status 枚举白名单；
  后端为最终防线覆盖表单/Excel 导入/拖拽全路径；导入遇非法行**跳过+警告**（不整批失败）；
  前端 U 位上限按所选机柜动态收紧（Playwright 真组件验证留痕）
- N-11 操作撤销（Ctrl+Z）、N-12 全局快捷键、N-13 拖拽增强（tooltip/下架确认）
- N-14 设备二维码标签与打印（7 字段多行键值契约，见 00-总览 §3.9）；**2026-09-16 标签改版**：仅显示二维码图片（去除品牌行与全部可读文字，信息全量承载于码内，扫码即得）
- **二维码微信兼容性说明**（主理人 2026-09-16 真机验证拍板：保持现状）：微信扫一扫对含中文的静态文本码提示"暂不支持展示二维码中的文本内容"，但**支持直接复制**——识别实际成功，仅展示层策略；手机相机/支付宝/QQ 均直接显示 7 字段完整信息。码内容维持中文明文契约不变
- **二维码标签打印修复**（`4b8d99f`）：打印媒体下二维码矢量放大 132px→185mm 并整页居中，@page 边距收至 10mm，消除 A4 大片留白
- N-15 四套主题预设（暗/亮/护眼绿/夜间蓝）+ 持久化
- N-17 HTML 报表增强、N-18 数据库备份/恢复、N-20 批量删除、N-21 导入向导

### 数据库

- schema **v5**（`80e5874`、`98dcc20`）：4 表加时间戳；`devices.deleted_at` 软删除；唯一索引重建为**部分索引**（`WHERE deleted_at IS NULL`，软删记录退出唯一约束）；新增 6 个索引。
- 多 U 设备下架后重上架退化为 1U 的缺陷修复（v4 `height_u` 固有高度列 + 四级优先级推导）。

### 缺陷修复

- 二维码安静区补至 4 模块（ISO/IEC 18004，`57ce44b`）；载荷 7 字段多行键值格式化（`97cd7db`）
- 恢复窗口按完整时长判定（`d7a8d91`）；报表模板对齐 Rust 结构体（`634d98f`）

### 文档

- 新增 `docs/00-项目资料总览.md`（单一事实来源）与 `docs/WORKBUDDY-新建项目提示词.md`
- **移除 4 份过时文档**（`59370bb`，主理人拍板）：现状盘点-2026-09-11、HANDOFF、analysis-架构分析报告、v1.1 产品介绍——结论已被代码与总览推翻，git 历史可查

### 工程基建

- Git 仓库建立（main），build.bat / check.bat 一键脚本（含 MSVC 自定位）
- husky pre-commit 门禁（cargo check/clippy/test + tsc + eslint + 离线红线）**已挂载并首次实跑**：cargo test 95/95 通过；钩子 ROOT 路径 bug 与 lint-check 空格路径 bug 修复
- CI workflow（`.github/workflows/ci.yml`，与 pre-commit 门禁对齐）
- **ESLint 依赖决策**：typescript-eslint@8 官方仅支持 TS <6.1.0，本项目 TS 7.0.2 触发硬门禁报错（上游 issue #10940 跟踪 TS ≥7.1 支持）。依赖暂不安装，`npm run lint` 改指向离线红线检查 `scripts/lint-check.mjs`；`.eslintrc.cjs` 保留，上游兼容后一键启用
- `lint-check.mjs` 白名单新增 2 条（撤销栈深拷贝收窄 ×3、AntD sorter 收窄 ×1）——**技术债：待重构为类型守卫**
- 版本号四处同步机制（Cargo.toml / tauri.conf.json / package.json / Layout 徽标）

## [1.2.0] - 2026-08 — 质量加固版

- 🐛 **P0 清零**：COALESCE 置空失效（下架功能坏）改三态 `Patch<T>`；CSP 策略启用；LIKE 通配符转义（`escape_like()`）；迁移事务保护（`with_migration_tx`）；7 索引 + 4 UNIQUE（schema v3）
- ⚡ 后端健壮性：连接池错误分类、`.ok()` 吞错清除、导出全异步化（UI 不冻结）、N+1 消除、`height_u` 固有高度（schema v4）
- 🧹 前端重构：RackView 879→133 行、DeviceList 577→252 行、LayoutContext 拆分、类型单一来源（12 重复接口 → 0）
- 📤 导出接线：机柜部署图 / 单机柜导出入口补齐
- 🔒 日志脱敏：导入日志不再记录序列号/资产编号明文

## [1.1.0] - 2026-06-10 — Tauri 重构版

- 🏗️ 从 Python FastAPI + pywebview 全面迁移到 Rust + Tauri 2
- ⚡ 安装包 -82%（100+MB → 18MB）、启动 6-10 倍（3-5s → <500ms）、内存 -80%
- 🔄 拖拽上下架（资源池↔机柜↔机柜）、U 位自动分配、三重查重导入
- ⚙️ 可开关日志（按天轮转 7 天）、Dark/Light 主题、参数化查询 + 软级联删除
