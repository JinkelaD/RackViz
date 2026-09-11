# RackViz v1.2 落地记录

> 状态：✅ 已执行（对应 `RackViz-v1.2-升级方案.md` 的 Phase 1–5 与「代码审查标准与流程」红线）
> 时间：v1.1 → v1.2 优化会话
> 项目路径：`D:\dsh-project\RackViz`

本文记录 v1.2 各轮实际落地的变更，作为交接文档与旧分析报告的增量说明。
旧文档（`analysis-架构分析报告.md` / `HANDOFF-项目交接文档.md` / `RackViz-v1.1-产品介绍.md`）
中的行号、指标为 v1.1 快照，与本记录冲突时以本文 + 实际代码为准。

---

## 第 1 轮：COALESCE 置空 P0 Bug（✅ 已修复）

**问题**：`Option<T>` 反序列化无法区分「字段缺失」与「显式 null」，`COALESCE(?n, col)`
导致「拖回资源池 / 未上架」等置空操作全部失效（下架功能实际是坏的）。

**修复**：
- `models.rs` 新增三态 `Patch<T>`：`Unset`(缺失=不改) / `Set(v)`(更新为值) / `Clear`(null=置 NULL)
- 4 个 `*Update` 结构的全部字段从 `Option<T>` 改为 `Patch<T>`（`#[serde(default)]`）
- 4 个 db `update_*` 函数从 COALESCE 全部改为**显式字段列表动态 SQL**（`db/mod.rs` 的 `patch_assign!` 宏）
- 日期字段：空串/`Clear` → NULL；格式无效 → validation 错误
- **28 处 COALESCE 全部移除**；新增 6 个回归测试（下架置空/部分更新/文本日期清空/坏日期等）

## 第 2 轮：安全与数据底线（✅ 已修复）

| 项 | 修复 |
|----|------|
| CSP | `tauri.conf.json` `csp: null` → 完整策略（script/style/img/font/connect 白名单 + object-src none 等） |
| 字体 | `index.html` 移除 Google Fonts CDN（本地系统字体栈回退，离线可用） |
| LIKE 通配符 | `db/devices.rs` 新增 `escape_like()`（转义 `\` `%` `_`）+ `ESCAPE '\'`，含回归测试 |
| 迁移 v3 | `migration.rs` 新增 `migrate_v2_to_v3`：7 个普通索引 + 4 个 UNIQUE 约束（重复数据先清理/跳过并告警） |
| 迁移事务 | 每个 `migrate_*` 由 `with_migration_tx()`（BEGIN IMMEDIATE/COMMIT/ROLLBACK）包裹，含回滚测试 |
| 日志脱敏 | `excel.rs` 导入跳过日志不再记录 serial_no/asset_no/名称明文，只记行号与已存在设备 id |

## 第 3 轮：后端健壮性（✅ 已修复）

| 项 | 修复 |
|----|------|
| 连接池样板 | `DbState::conn()` + 33 处 `pool.get().map_err(AppError::io)` → `state.conn()?`（自动 From 转换） |
| `.ok()` 吞错 | `db/settings.rs`、`db/devices.rs` find 系改用 `OptionalExtension::optional()?`；delete 日志改显式 match |
| insert 后 unwrap | 4 个 `insert_*` 的 `get().map(\|r\| r.unwrap())` → `?.ok_or_else(not_found)` |
| find_or_create 竞态 | `db/racks.rs`/`device_models.rs`：先查 → INSERT → 捕获 ConstraintViolation 重查 |
| N+1 查询 | `excel.rs export_racks_excel`：per-rack 查设备 → 一次全量 + 内存按 rack 分组 |
| 导出异步化 | `commands/exports.rs` 全部命令 async：数据生成与落盘在 `spawn_blocking`，对话框非阻塞（save_file + channel） |
| 启动/日志 | `lib.rs` 去掉 `.expect()`/`to_str().unwrap()`；`logging.rs` Mutex poison 容忍 |
| 无用依赖 | Cargo.toml 移除 `tauri-plugin-shell`/`dirs`/dependencies 段 `tauri-build`/前端未用的 `tauri-plugin-fs` 注册与 `fs:*` `shell:default` 权限 |

## 第 4 轮：前端重构（✅ 组件 < 300 行）

| 项 | 修复 |
|----|------|
| Context 拆分 | `LayoutContext` 23 字段 → `ViewContext`(9) + `RoomContext`(6)，RoomTabs 自包含（用 hooks 而非 props） |
| 组件拆分 | `RackView` 879→120 行、`DeviceList` 577→229 行、`Layout` 252→142 行；全部子组件 < 200 行 |
| 新组件 | `components/rack/`：RackCard / DeviceStockPanel / AddRackModal / EditRackModal / EditDeviceModal |
| 新组件 | `components/device/`：DeviceFormModal / ModelManageModal / ResizableTitle / deviceColumns |
| 状态下沉 | `hooks/useRackView.ts`（页面逻辑整体收敛） |
| 类型单一来源 | `tauri-api.ts` 重复的 12 个实体/请求接口全部改为从 `types/index.ts` 派生（Partial + 必填 name） |
| 性能 | 机柜/型号/机房/设备查找 Map 化（useMemo 稳定引用）；过滤与列定义 useMemo |
| 竞态 | `useApiList` 乐观更新改函数式 setState + itemsRef 快照回滚 |
| 非空断言 | `device.end_u!` 等 → `?? 0`（RackCard） |

## 第 5 轮：收尾与质量基建（✅ 已配置）

| 项 | 落地 |
|----|------|
| 版本号 | 统一为 **1.2.0**（Cargo.toml / tauri.conf.json / frontend package.json / Layout 徽标 v1.2） |
| 导出接线 | 机柜部署图 + 单机柜导出：StatusBar 增加「部署图」「单机柜」按钮（RackView/useRackView 接线） |
| Clippy | `Cargo.toml [lints.clippy]`：unwrap_used / expect_used / panic / print_stdout / print_stderr = warn；生产代码 0 警告 |
| ESLint | `frontend/.eslintrc.cjs` + package.json `lint` script（依赖安装后生效） |
| 离线红线检查 | `frontend/scripts/lint-check.mjs`（零依赖，npm run lint:offline） |
| pre-commit | `.husky/pre-commit` 钩子 + 根目录 `check.bat` 一键检查 |

## 验证结果（全部通过）

- `cargo test --offline`：**29 个测试全部通过**
- `cargo check / clippy`：**0 错误 0 警告（生产代码）**
- `npx tsc --noEmit / tsc -b`：**0 错误**
- `node scripts/lint-check.mjs`：**0 违规**

## 遗留说明

- `--all-targets` 下 clippy 会对**测试代码**报 unwrap 警告（测试惯用法，审查规范允许）；
- 前端 ESLint 依赖（typescript-eslint / react-hooks）需联网 `npm i -D` 后启用，离线请用 `npm run lint:offline`；
- 构建环境：本机 cargo 无法联网，使用工作区 `.cargo-home/` 缓存离线构建（见 .gitignore 注释）；
- 数据库迁移：老用户库启动时自动 v2 → v3（索引/约束/清理重复序列号）。

---

## 补充修复：多U设备下架后重上架退化为 1U（✅ 已修复）

**问题**：设备占用高度此前只隐式存在于 `start_u..end_u`（上架区间）。
下架 / 拖回资源池 / 未上架时 `start_u/end_u` 被清空，高度信息随之丢失；
重上架时前端回退到 `model?.height_u || 1` —— 无型号（如 Excel 导入）或
型号高度与实占高度不一致的设备会按 1U 上架，占用与实际不符。

**修复**（迁移 v4 + 前后端联动）：
- `migration.rs` 新增 `migrate_v3_to_v4`：`devices` 增加持久字段 `height_u`（固有高度）
  并回填：在架设备按实际占用区间 `end_u - start_u + 1`；未上架但有型号的按型号高度；
  其余保持 1。`CURRENT_VERSION = 4`，迁移链 0/1/2/3 均续接 v4。
- `models.rs`：`Device` 增 `height_u`；`DeviceCreate.height_u: Option<i32>`；
  `DeviceUpdate.height_u: Patch<i32>`（服务端按 U 位区间/型号推导，杜绝前端遗漏）。
- `db/devices.rs`：
  - 全部 SELECT / INSERT / `row_to_device` 纳入 `height_u`（共用 `DEVICE_SELECT` 常量）；
  - 新增 `resolve_device_height()`（显式值 > U 位区间 > 型号高度 > 1）；
  - `update_device`：显式 Set 写值；Unset 且本次更新落在有效 U 位区间时自动同步区间高度；
    Clear（NULL）被校验拒绝（列 NOT NULL）。
- `db/racks.rs`：`delete_rack` 改为同时清空 `start_u/end_u`（彻底下架），高度保留在 `height_u`。
- 前端：`types/index.ts` Device 增 `height_u`；`useRackView` 新增 `resolveDeviceHeight()`
  （区间 > 设备固有高度 > 型号 > 1），上架/移动 payload 显式带 `height_u`；
  `DeviceStockPanel` 资源池按固有高度显示 U 数。
- 新增回归测试：`test_insert_derives_height_from_u_span`、
  `test_height_persists_after_unrack_and_rerack`（下架清位后 `height_u` 仍为 3，
  重上架到新位置仍按 3U）、`test_v3_to_v4_backfills_device_height`（含在架/资源池两态回填）。

**验证**：`cargo test --offline` 32 个测试全部通过；clippy（生产代码）0 警告；
`tsc --noEmit / tsc -b` 0 错误；红线检查 0 违规；前端 dist 与 `rackviz.exe` 已重新编译。

**注意**：本次仅新增 `height_u` 列（ALTER TABLE + DEFAULT 1），老库启动自动升 v4，无手工操作。
