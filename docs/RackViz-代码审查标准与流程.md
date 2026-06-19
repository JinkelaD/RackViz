# RackViz 代码审查标准与流程规范

> 版本：v1.0  
> 生效日期：2026-06-10  
> 适用范围：RackViz 全项目（Rust + Tauri 后端 + React 前端）

---

## 一、审查目标

代码审查不是"挑毛病"，而是**集体知识传递和风险预防机制**。对 RackViz 而言，核心目标：

1. **守住安全底线** — v1.1 审计发现 CSP 关闭、LIKE 通配符未转义等问题，审查必须拦截同类风险
2. **保证数据正确性** — COALESCE 无法置空字段等逻辑 Bug 必须在审查中发现
3. **控制技术债务增长** — 代码重复（20+ 处连接池样板、6 处 SELECT 列列表）、God Component 等需要遏制
4. **提升团队认知** — 每次审查都是技术决策的上下文传递

---

## 二、严重级别定义

### 🔴 P0 — 阻断合并（Blocker）

**定义**：可能导致数据丢失、安全漏洞、应用崩溃的问题。合并前必须修复。

| 类别 | 示例 |
|------|------|
| **安全漏洞** | SQL 注入、XSS、敏感信息泄露到日志、CSP 缺失 |
| **数据正确性** | 更新无法置空字段、竞态条件导致数据不一致、非原子批量操作 |
| **运行时崩溃** | `.unwrap()` 在可能为 None 的值上、panic 路径、NaN 渗透到 CSS |
| **功能回归** | 拖拽失效、导入重复、核心流程断裂 |

### 🟡 P1 — 建议修复（Important）

**定义**：不阻断合并但应尽快修复。影响可维护性、性能或用户体验。

| 类别 | 示例 |
|------|------|
| **错误处理缺陷** | `.ok()` 吞掉错误、`console.error` 无用户提示、乐观更新回滚逻辑错误 |
| **类型安全** | `as T` 强制断言、`!` 非空断言无保护、`any` 类型 |
| **代码重复** | 相同逻辑出现 3+ 处、SELECT 列列表硬编码重复 |
| **性能隐患** | N+1 查询、缺少索引、内联闭包导致不必要重渲染 |
| **组件设计** | God Component（>300 行）、Prop Drilling 超过 2 层 |

### 💭 P2 — 建议改进（Nit）

**定义**：锦上添花，不阻塞也不急于修复。

| 类别 | 示例 |
|------|------|
| **代码风格** | 命名不一致、注释缺失、魔数未提取常量 |
| **次要重构** | 内联 SVG 提取为组件、Modal 结构复用 |
| **体验优化** | 日期格式化、快捷键支持 |

---

## 三、Rust 后端审查清单

### 3.1 安全性（每条必查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| S-1 | SQL 使用参数化查询 | 搜索 `format!` 中的 SQL | `format!("WHERE name = '{}'", name)` | `WHERE name = ?1` |
| S-2 | LIKE 通配符已转义 | 搜索 `LIKE` 和 `%{` | `format!("%{}%", input)` | `format!("%{}%", escape_like(input))` |
| S-3 | 无 `.unwrap()` 在生产路径 | `rg '\.unwrap\(\)' src/` 排除测试 | `conn.execute(...).unwrap()` | `conn.execute(...).map_err(...)?` |
| S-4 | 敏感信息不记录到日志 | 搜索 `log::info` 中的变量 | `log::info!("sn={}", serial_no)` | `log::info!("id={}", id)` |
| S-5 | CSP 策略已配置 | 检查 `tauri.conf.json` | `"csp": null` | `"csp": "default-src 'self'"` |

### 3.2 数据正确性（每条必查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| D-1 | COALESCE 不阻止字段置空 | 搜索 `COALESCE` | `SET rack_id = COALESCE(?1, rack_id)` | 显式字段列表或 `clear_fields` 参数 |
| D-2 | 批量操作在事务中 | 搜索循环内的 DB 操作 | `for item in items { conn.execute(INSERT) }` | `with_transaction(conn, \|\| { for item in items { ... } })` |
| D-3 | 原子性顺序操作有回滚 | 检查连续 await DB 操作 | `update(a); update(b);` // b 失败无回滚 | 事务包裹或补偿逻辑 |
| D-4 | UNIQUE 约束存在于唯一字段 | 检查 migration.rs | `serial_no TEXT DEFAULT ''` | `serial_no TEXT DEFAULT '' UNIQUE` |
| D-5 | `find_or_create` 使用 ON CONFLICT | 搜索 `find_or_create` | `SELECT → if none → INSERT` | `INSERT ON CONFLICT DO NOTHING RETURNING id` |

### 3.3 错误处理（每条必查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| E-1 | 连接池错误用 `?` 自动转换 | 搜索 `pool.get().map_err` | `pool.get().map_err(\|e\| AppError::io(...))` | `pool.get()?` // 利用 `From` trait |
| E-2 | 不用 `.ok()` 吞掉 DB 错误 | 搜索 `.ok()` | `get_device(&conn, id).ok().flatten()` | `get_device(&conn, id)?.unwrap_or_default()` |
| E-3 | `insert_*` 返回值安全处理 | 搜索 `last_insert_rowid` | `get(conn, id).map(\|r\| r.unwrap())` | `get(conn, id)?.ok_or(AppError::not_found(...))` |
| E-4 | ROLLBACK 失败至少记录日志 | 搜索 `let _ = ... ROLLBACK` | `let _ = conn.execute_batch("ROLLBACK");` | `if let Err(e) = conn.execute_batch("ROLLBACK") { log::error!(...) }` |
| E-5 | AppError 实现了 `std::error::Error` | 检查 error.rs | 只有 `Display` | 同时实现 `std::error::Error` |

### 3.4 代码质量（建议检查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| Q-1 | SELECT 列列表不硬编码 | 搜索重复的 `"id, name, ..."` 列表 | 6 处相同列字符串 | 提取为 `const DEVICE_COLUMNS: &str` 或宏 |
| Q-2 | `row_to_*` 映射函数统一 | 检查 `list_*` 和 `get_*` 的闭包 | 两个函数各自内联映射闭包 | 提取为 `fn row_to_device(row: &Row) -> Result<Device>` |
| Q-3 | 输入验证全面 | 检查 `create_*` 命令 | 只验证 name 非空 | 验证 name、IP 格式、U 位范围、枚举值、外键存在性 |
| Q-4 | 日期解析函数不重复 | 搜索 `parse_*_date` | 两个文件各写一个 | 提取到 `utils.rs` 共用 |
| Q-5 | 连接池获取抽取为方法 | 搜索 `pool.get()` | 每个 command 重复 | `impl DbState { fn conn(&self) -> Result<Connection> { ... } }` |
| Q-6 | 测试 `setup_db()` 不重复 | 搜索 `setup_db` | 4 个文件各写一份 | 提取到 `test_utils.rs` |

### 3.5 性能（建议检查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| P-1 | 新增查询有对应索引 | 检查 WHERE 子句 | `WHERE serial_no = ?` 无索引 | 迁移中添加 `CREATE INDEX` |
| P-2 | 无 N+1 查询 | 检查循环内的 DB 查询 | `for rack in racks { list_devices(conn, rack.id) }` | 一次查全部，内存中 `group_by` |
| P-3 | 只查所需列 | 检查 `list_*_map` 函数 | `list_racks` → 取 `name` | `SELECT id, name FROM racks` |
| P-4 | 导出操作不阻塞 UI | 搜索 `blocking_` | `blocking_save_file()` | 异步对话框 + 后台线程 |

---

## 四、React 前端审查清单

### 4.1 类型安全（每条必查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| T-1 | 不使用 `as T` 掩盖结构缺失 | 搜索 `as T`、`as Device` 等 | `{ ...d, ...data } as T` | 类型守卫或 runtime 校验 |
| T-2 | nullable 字段有 null 保护 | 搜索 `!` 非空断言 | `device.start_u!` | `device.start_u ?? 0` 或 null 检查 |
| T-3 | 不使用 `any` 类型 | `rg ': any' src/` | `Record<string, any>` | 具体类型或泛型 |
| T-4 | 类型定义不重复 | 比较 `types/` 和 `tauri-api.ts` | 两份 `Device` 类型 | 单一来源，API 类型继承 domain 类型 |
| T-5 | 前后端类型保持同步 | 比较前端 `types` 和 Rust `models` | 手动维护两份 | 使用 `ts-rs` 或 `specta` 自动生成 |

### 4.2 错误处理（每条必查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| E-1 | 所有 invoke 调用有 try/catch | 搜索 `await.*invoke\|await.*api\.` | `await update(id, data)` | `try { await update(id, data) } catch { message.error(...) }` |
| E-2 | 错误对用户可见 | 搜索 `console.error` | `.catch(console.error)` | `.catch(err => { message.error('操作失败'); console.error(err) })` |
| E-3 | 乐观更新回滚正确 | 检查 `useApiList` catch 块 | `setItems(prev); refresh();` | 先回滚，延迟再 refresh，或使用 ref |
| E-4 | 顺序写操作有原子保护 | 检查连续 `await` | `await updateA(); await updateB();` | 事务或补偿逻辑 |

### 4.3 组件设计（建议检查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| C-1 | 单组件 < 300 行 | 检查文件行数 | RackView.tsx 880 行 | 拆分为 RackGrid、DeviceBlock、EditModal 等 |
| C-2 | useState < 8 个/组件 | 计算 `useState` 数量 | 12 个 useState | 相关状态用 `useReducer` 分组 |
| C-3 | Context 接口 < 10 个字段 | 检查 Context 类型 | 17 个属性 | 拆分为 ThemeContext + RoomContext + RackContext |
| C-4 | Prop drilling ≤ 2 层 | 追踪 props 传递链 | Layout → RackView → RoomTabs (10 props) | 专用 Context 或组合模式 |
| C-5 | 类型接口不重复定义 | 比较跨文件 ContextType | RackView 复制了 Layout 的 ContextType | `import type { LayoutContext } from ...` |
| C-6 | Modal 提取为独立组件 | 检查内联 Modal | 页面内 60 行 Modal JSX | `<DeviceEditModal />` 独立组件 |

### 4.4 性能（建议检查）

| # | 检查项 | 审查方式 | 不合格示例 | 合格示例 |
|---|--------|---------|-----------|---------|
| P-1 | 列表过滤用 `useMemo` | 检查 `.filter(` 无 useMemo | `const filtered = items.filter(...)` | `const filtered = useMemo(() => items.filter(...), [items, ...])` |
| P-2 | 列定义不每次渲染重建 | 检查 `ColumnsType` 在组件内 | `const cols: ColumnsType = [...]` 在组件体 | 提取到组件外或 `useMemo` |
| P-3 | useCallback 依赖稳定 | 检查 `[items]` 依赖 | `useCallback(fn, [items])` | `useRef` 存储 items，依赖数组为 `[]` |
| P-4 | 内联闭包不大量创建 | 检查列表中的内联函数 | `onClick={() => handleClick(id)}` | `data-*` 属性 + 事件委托或 `useCallback` |
| P-5 | `useMemo` 依赖列表完整 | 检查 useMemo 第二个参数 | 缺少内部使用的函数引用 | 所有使用的变量/函数都在依赖中 |

---

## 五、数据库变更审查清单

| # | 检查项 | 审查方式 | 必要性 |
|---|--------|---------|--------|
| M-1 | 新增表有 `created_at` / `updated_at` | 检查 CREATE TABLE | 必须 |
| M-2 | 新增查询字段有对应索引 | 检查 WHERE/JOIN 条件 | 必须 |
| M-3 | 唯一性字段有 UNIQUE 约束 | 检查业务唯一字段 | 必须 |
| M-4 | 迁移脚本使用事务 | 检查 `BEGIN`/`COMMIT` | 必须 |
| M-5 | 迁移版本号递增 | 检查 `PRAGMA user_version` | 必须 |
| M-6 | 外键约束已启用 | 检查 `PRAGMA foreign_keys` | 必须 |
| M-7 | 破坏性变更（删列/改类型）有数据迁移 | 检查迁移脚本 | 必须 |
| M-8 | 向后兼容（旧版本不崩溃） | 考虑降级场景 | 建议 |

---

## 六、审查流程

### 6.1 流程概览

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  开发完成     │───►│  自查清单     │───►│  提交 PR     │───►│  审查者审查   │
│  编译+测试通过 │    │  (10分钟)    │    │              │    │  (30-60分钟) │
└─────────────┘    └─────────────┘    └─────────────┘    └──────┬──────┘
                                                                │
                                              ┌─────────────────┼─────────────────┐
                                              │                 │                 │
                                        ┌─────▼─────┐   ┌─────▼─────┐   ┌─────▼─────┐
                                        │ P0 发现    │   │ 仅 P1/P2  │   │ 无问题    │
                                        │ 退回修复   │   │ 批准+追踪  │   │ 直接合并  │
                                        └───────────┘   └───────────┘   └───────────┘
```

### 6.2 角色与职责

| 角色 | 职责 | 审查重点 |
|------|------|---------|
| **提交者** | 编写代码、自查、回复审查意见 | 自查清单、编译测试 |
| **审查者** | 审查代码质量、提出意见 | 安全性、正确性、架构 |
| **架构师**（大型变更） | 评估架构影响 | 分层是否合理、接口契约 |

### 6.3 审查前：提交者自查（10 分钟）

提交 PR 前，提交者必须完成以下自查：

**Rust 后端自查**：
```bash
# 1. 编译检查
cargo check 2>&1 | head -20

# 2. 单元测试
cargo test 2>&1 | tail -5

# 3. Clippy 静态分析
cargo clippy -- -D warnings 2>&1 | head -20

# 4. 检查 unwrap 使用
rg '\.unwrap\(\)' src-tauri/src/ --glob '!*test*'

# 5. 检查 SQL 安全性
rg 'format!.*SELECT\|format!.*INSERT\|format!.*UPDATE\|format!.*DELETE' src-tauri/src/

# 6. 检查 LIKE 通配符
rg 'LIKE' src-tauri/src/

# 7. 检查日志敏感信息
rg 'log::info.*serial_no\|log::info.*asset_no\|log::info.*ip_address' src-tauri/src/
```

**React 前端自查**：
```bash
# 1. 类型检查
npx tsc --noEmit 2>&1 | head -20

# 2. ESLint
npx eslint src/ 2>&1 | head -20

# 3. 检查 any 类型
rg ': any' frontend/src/

# 4. 检查非空断言
rg '\w+!' frontend/src/ --glob '!*.d.ts'

# 5. 检查 console.error
rg 'console\.error' frontend/src/

# 6. 检查裸 invoke 调用（无 try/catch）
rg 'await.*invoke\(' frontend/src/ | grep -v 'try'
```

### 6.4 审查中：审查者审查（30-60 分钟）

#### Step 1：快速扫描（5 分钟）

1. **看 PR 描述**：改了什么、为什么改、影响范围
2. **看文件列表**：改了多少文件、涉及哪些模块
3. **看 diff 行数**：> 500 行需要拆分

#### Step 2：逐文件审查（20-40 分钟）

按优先级审查：

1. **安全相关文件**（优先）
   - `migration.rs` — SQL 安全性、索引
   - `tauri.conf.json` — CSP、权限
   - `commands/*.rs` — 输入验证、错误处理
   - `excel.rs` — 导入逻辑、数据完整性

2. **正确性相关文件**
   - `db/*.rs` — SQL 逻辑、事务、COALESCE
   - `hooks/*.ts` — 状态管理、竞态条件
   - `tauri-api.ts` — 类型安全、错误处理

3. **质量相关文件**
   - 组件文件 — 拆分、性能
   - 样式文件 — 主题兼容

#### Step 3：填写审查意见

每条意见使用以下格式：

```
🔴 **[安全] LIKE 通配符未转义**
db/devices.rs:38 — `format!("%{}%", s)` 中用户输入的 `%` 和 `_` 未转义

**影响**：用户搜索 `100%` 会匹配包含 `100` 的所有字符串

**建议**：添加 `escape_like()` 函数，或使用 `\\%` / `\\_` 转义
```

### 6.5 审查后：问题追踪

| 发现级别 | 处理方式 | 追踪 |
|---------|---------|------|
| 🔴 P0 | 退回提交者修复，合并前必须解决 | PR 标记 `needs-fix` |
| 🟡 P1 | 批准合并，但创建 Issue 追踪 | 新建 Issue，标签 `tech-debt` |
| 💭 P2 | 记录在 PR 评论中，不强制修复 | 无需 Issue |

---

## 七、审查模板

### 7.1 PR 描述模板

```markdown
## 变更类型
- [ ] 🆕 新功能
- [ ] 🐛 Bug 修复
- [ ] 🔒 安全修复
- [ ] ♻️ 重构
- [ ] 📦 依赖更新
- [ ] 🗃️ 数据库变更

## 变更说明
<!-- 简述做了什么、为什么做 -->

## 影响范围
<!-- 列出受影响的模块/功能 -->

## 自查结果
- [ ] `cargo check` / `tsc --noEmit` 通过
- [ ] `cargo test` / 前端测试通过
- [ ] `cargo clippy` / `eslint` 通过
- [ ] 无 `.unwrap()` 在生产代码中
- [ ] 新增 SQL 查询有对应索引
- [ ] 所有 invoke 调用有错误处理
- [ ] 无敏感信息记录到日志

## 截图/录屏
<!-- UI 变更请附截图 -->
```

### 7.2 审查意见模板

```
🔴/🟡/💭 **[类别] 简述**
文件:行号 — 具体代码引用

**影响**：为什么这是个问题

**建议**：修复方案或替代方案
```

### 7.3 审查总结模板

```markdown
## 审查总结

**总体评价**：👍 可以合并 / 👎 需要修改 / ⚠️ 有条件合并

**统计**：
- 🔴 P0: X 个（必须修复）
- 🟡 P1: X 个（建议修复，已建 Issue #N）
- 💭 P2: X 个（记录）

**亮点**：
- <!-- 值得肯定的设计决策或实现 -->

**关键问题**：
- <!-- P0 问题摘要 -->

**后续建议**：
- <!-- 非本次 PR 范围但值得关注的改进方向 -->
```

---

## 八、自动化检查配置

### 8.1 Rust — Clippy 配置

在 `src-tauri/.clippy.toml` 或 `Cargo.toml` 中配置：

```toml
# Cargo.toml
[lints.clippy]
unwrap_used = "warn"           # 生产代码中的 unwrap
expect_used = "warn"           # 生产代码中的 expect  
indexing_slicing = "allow"     # 暂不强制
panic = "warn"                 # 可能 panic 的代码
print_stderr = "warn"          # 使用 log 替代 print
print_stdout = "warn"          # 使用 log 替代 print
```

### 8.2 TypeScript — ESLint 配置

在 `frontend/.eslintrc.json` 中添加：

```json
{
  "rules": {
    "@typescript-eslint/no-explicit-any": "error",
    "@typescript-eslint/no-non-null-assertion": "warn",
    "@typescript-eslint/no-unsafe-assignment": "warn",
    "@typescript-eslint/no-unsafe-member-access": "warn",
    "no-console": ["warn", { "allow": ["warn", "error"] }],
    "react-hooks/exhaustive-deps": "warn"
  }
}
```

### 8.3 Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "🔍 运行预提交检查..."

# Rust 检查
cd src-tauri
cargo check --quiet 2>&1
if [ $? -ne 0 ]; then echo "❌ cargo check 失败"; exit 1; fi
cargo clippy --quiet -- -D warnings 2>&1
if [ $? -ne 0 ]; then echo "❌ clippy 发现问题"; exit 1; fi
cargo test --quiet 2>&1
if [ $? -ne 0 ]; then echo "❌ 测试失败"; exit 1; fi
cd ..

# 前端检查
cd frontend
npx tsc --noEmit --pretty 2>&1
if [ $? -ne 0 ]; then echo "❌ TypeScript 类型检查失败"; exit 1; fi
npx eslint src/ --quiet 2>&1
if [ $? -ne 0 ]; then echo "❌ ESLint 发现问题"; exit 1; fi
cd ..

echo "✅ 预提交检查通过"
```

---

## 九、项目特定审查红线

基于 v1.1 审计发现的**项目特定高风险模式**，以下情况必须标记为 P0：

### 9.1 Rust 后端红线

| 红线 | 背景 | 审查方式 |
|------|------|---------|
| 新增 `COALESCE(?N, col)` 更新 | v1.1 P0：无法置空字段 | 禁止新增 COALESCE 更新，改用显式字段列表 |
| `pool.get().map_err(AppError::io)` | 错误类型误分类为 IoError | 必须用 `?` 自动转换（已有 `From` trait） |
| `last_insert_rowid() + get() + unwrap()` | 可能 panic | 改用 `RETURNING` 子句或 `ok_or` |
| `blocking_save_file()` | 阻塞 UI 线程 | 使用异步对话框 |
| 新增无索引的 WHERE 查询 | v1.1 P0：缺少索引 | 同时添加迁移创建索引 |
| 日志中记录 serial_no/asset_no | 敏感信息泄露 | 只记录 id |

### 9.2 React 前端红线

| 红线 | 背景 | 审查方式 |
|------|------|---------|
| 裸 `await invoke()` 无 try/catch | 用户无错误反馈 | 必须包裹 try/catch + 用户提示 |
| `device.start_u!` / `end_u!` | NaN 渗透到 CSS | 必须有 null 保护 |
| `{ ...d, ...data } as T` 乐观更新 | 可能遗漏必填字段 | 使用类型守卫或 runtime 校验 |
| `useCallback([items])` | 依赖不稳定导致无限刷新 | 使用 useRef 稳定化 |
| 组件超过 300 行 | God Component | 必须拆分 |
| 重复定义 ContextType | 类型不同步风险 | 必须导入而非复制 |

### 9.3 数据库红线

| 红线 | 背景 | 审查方式 |
|------|------|---------|
| 新增表无 `created_at` | v1.1 P1：缺少时间戳 | CREATE TABLE 必须包含 |
| 唯一字段无 UNIQUE 约束 | v1.1 P1：只有代码层校验 | migration 必须添加 |
| 迁移脚本无事务 | 中途失败状态不一致 | 必须用 `with_transaction` |
| 新增表无索引 | v1.1 P0 | WHERE/JOIN 字段必须建索引 |

---

## 十、审查节奏与规模指南

### 10.1 审查规模

| PR 大小 | 行数 | 建议审查时间 | 建议 |
|---------|------|-------------|------|
| 小 | < 100 行 | 15 分钟 | 一次性审完 |
| 中 | 100-300 行 | 30 分钟 | 一次性审完 |
| 大 | 300-500 行 | 45-60 分钟 | 分两次审 |
| 超大 | > 500 行 | — | **要求拆分 PR** |

### 10.2 审查频率

| 场景 | 建议频率 |
|------|---------|
| 日常开发 | 每个 PR 审查 |
| 紧急修复 | 合并后补审 |
| 大型重构 | 设计评审 + 分阶段代码审查 |
| 版本发布前 | 全量审计（如 v1.1 审计） |

### 10.3 审查者轮换

避免同一个人总审查同一个人的代码：

- 两人团队：交叉审查
- 功能交叉：前端改后端审，后端改前端审
- 安全相关变更：必须由第二人审查

---

## 十一、指标与度量

### 11.1 审查质量指标

| 指标 | 目标 | 度量方式 |
|------|------|---------|
| PR 审查覆盖率 | 100% | 审查过的 PR / 总 PR |
| P0 发现率 | < 5% | 含 P0 的 PR / 总 PR |
| 审查响应时间 | < 4h | 提交到首次审查 |
| P0 修复时间 | < 24h | 发现到修复 |
| 自查通过率 | > 90% | 一次通过的 PR / 总 PR |

### 11.2 代码质量趋势

每月统计以下指标，追踪改进趋势：

| 指标 | v1.1 基线 | v1.2 目标 |
|------|----------|----------|
| Rust `.unwrap()` 数量 | ~10 | < 3 |
| 前端 `any` 类型数量 | 1 | 0 |
| 前端 `as T` 断言数量 | ~8 | < 3 |
| 代码重复函数数量 | ~15 | < 5 |
| 缺少索引的查询数量 | 7 | 0 |
| 无错误处理的 invoke 数量 | ~10 | 0 |
| 单元测试覆盖率（Rust） | ~40% | > 70% |
| 组件平均行数 | ~350 | < 250 |

---

## 附录 A：v1.1 审计问题 → 审查规则映射

| 审计问题 | 级别 | 对应审查规则 | 红线 |
|---------|------|-------------|------|
| CSP 关闭 | P0 | S-5 | ✅ |
| 缺少索引 | P0 | P-1、M-2 | ✅ |
| COALESCE 无法置空 | P0 | D-1 | ✅ |
| LIKE 通配符未转义 | P0 | S-2 | ✅ |
| `.unwrap()` 可能 panic | P0 | S-3、E-3 | ✅ |
| 错误类型误分类 | P1 | E-1 | ✅ |
| `.ok()` 吞掉错误 | P1 | E-2 | — |
| 前端无全局错误处理 | P1 | E-1、E-2 | ✅ |
| 非空断言 `!` | P1 | T-2 | ✅ |
| `as T` 强制断言 | P1 | T-1 | ✅ |
| God Component | P1 | C-1 | ✅ |
| 代码重复 | P2 | Q-1~Q-6 | — |

## 附录 B：常用审查命令速查

```bash
# === Rust 后端 ===
# 编译 + 测试
cargo check && cargo test

# Clippy 静态分析
cargo clippy -- -D warnings

# 检查 unwrap
rg '\.unwrap\(\)' src-tauri/src/ --glob '!*test*'

# 检查 SQL 拼接
rg 'format!.*SELECT\|format!.*INSERT\|format!.*UPDATE\|format!.*DELETE' src-tauri/src/

# 检查 COALESCE
rg 'COALESCE' src-tauri/src/

# 检查 blocking 调用
rg 'blocking_' src-tauri/src/

# 检查日志敏感信息
rg 'log::info.*serial_no\|log::info.*asset_no' src-tauri/src/

# === React 前端 ===
# 类型检查
npx tsc --noEmit

# ESLint
npx eslint src/

# 检查 any 类型
rg ': any' frontend/src/

# 检查非空断言
rg '\w+!' frontend/src/ --glob '!*.d.ts'

# 检查无 try/catch 的 invoke
rg -n 'await.*(invoke|api\.)' frontend/src/ | grep -v 'try'

# 检查 console.error（应改为用户提示）
rg 'console\.error' frontend/src/

# 检查 useState 数量
rg 'useState' frontend/src/ | wc -l

# === 数据库 ===
# 检查迁移中的索引
rg 'CREATE INDEX' src-tauri/src/

# 检查 UNIQUE 约束
rg 'UNIQUE' src-tauri/src/
```

---

*本标准由 RackViz v1.1 代码审计驱动制定，将随项目演进而持续更新。*
