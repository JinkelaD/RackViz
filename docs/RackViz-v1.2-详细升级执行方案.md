# RackViz v1.2 详细升级执行方案

> 方案版本：v2.0（基于实际代码扫描）
> 制定日期：2026-06-19
> 依据文档：`RackViz-项目全面梳理报告.md`
> 升级目标：v1.1 (5.4/10) → v1.2 (8.0/10)
> 预计工期：核心 13 天 + 功能增强 8 天 = 21 天

---

## 一、升级目标量化

| 维度 | v1.1 现状 | v1.2 目标 | 关键指标 |
|------|:---:|:---:|----------|
| **安全性** | 4/10 | 9/10 | CSP 非 null，LIKE 转义，敏感信息脱敏 |
| **数据正确性** | 5/10 | 9/10 | 无 COALESCE，有 UNIQUE + 索引 + 事务 |
| **错误处理** | 4/10 | 8/10 | 无 unwrap/expect/ok()吞错，invoke 全 try/catch |
| **代码质量** | 6/10 | 8/10 | 类型统一，组件 < 300 行，useState < 8 |
| **性能** | 5/10 | 8/10 | 有索引，useMemo，列定义提取 |
| **综合** | **5.4/10** | **8.0/10** | P0=0, P1<10 |

---

## 二、Phase 0：准备阶段（0.5 天）

### P0-01：创建升级分支

**操作步骤**：
```bash
cd /workspace
git checkout -b develop
git push -u origin develop
```

**验收**：`git branch` 显示 develop 分支存在，`git log` 与 main 一致。

### P0-02：统一版本号为 v1.2.0

**修改文件（4 处）**：

| 文件 | 位置 | 当前值 | 目标值 |
|------|------|--------|--------|
| `src-tauri/Cargo.toml` | `version` 字段 | `0.2.0` | `1.2.0` |
| `src-tauri/tauri.conf.json` | `version` 字段 | `0.2.0` | `1.2.0` |
| `frontend/package.json` | `version` 字段 | `1.0.0` | `1.2.0` |
| `frontend/src/components/Layout.tsx` | UI 显示版本号 | `v1.1` | `v1.2.0` |

**验收**：`grep -r "version" src-tauri/Cargo.toml src-tauri/tauri.conf.json frontend/package.json` 三处均为 `1.2.0`，UI 显示 `v1.2.0`。

### P0-03：配置 Rust Clippy 规则

**创建文件**：`src-tauri/.clippy.toml`

```toml
cognitive-complexity-threshold = 30
```

**修改文件**：`src-tauri/Cargo.toml`，在 `[lints.clippy]` 添加：

```toml
[lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
ok_expect = "warn"
```

**验收**：`cd src-tauri && cargo clippy 2>&1 | grep "unwrap_used\|expect_used"` 显示所有违规位置。

### P0-04：配置 ESLint 规则

**创建文件**：`frontend/.eslintrc.cjs`

```javascript
module.exports = {
  env: { browser: true, es2020: true },
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:react-hooks/recommended',
  ],
  parser: '@typescript-eslint/parser',
  parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
  plugins: ['react-refresh'],
  rules: {
    '@typescript-eslint/no-explicit-any': 'error',
    '@typescript-eslint/no-non-null-assertion': 'error',
    '@typescript-eslint/no-unnecessary-type-assertion': 'warn',
    'no-console': ['warn', { allow: ['warn', 'error'] }],
  },
};
```

**验收**：`cd frontend && npm run lint` 输出违规位置，但不阻塞构建。

### P0-05：配置 pre-commit hook

**操作步骤**：
```bash
cd frontend && npm install husky lint-staged --save-dev
npx husky init
```

**创建文件**：`.husky/pre-commit`

```bash
#!/bin/sh
cd src-tauri && cargo check --quiet 2>&1
cd src-tauri && cargo clippy --quiet 2>&1 | grep -E "error|unwrap_used|expect_used"
cd frontend && npx tsc --noEmit
cd frontend && npx eslint src/ --max-warnings 0
```

**验收**：提交时自动触发检查，违规会阻止提交。

### P0-06：备份当前数据库

**操作步骤**：
```bash
# 文档记录：提醒用户升级前手动备份
echo "升级前请备份：copy %APPDATA%/RackViz/rackviz.db rackviz.db.bak"
```

**验收**：升级方案文档中包含备份提醒。

---

## 三、Phase 1：安全与数据底线（3 天，P0 全清）

### 任务 S-01：配置 CSP 安全策略（0.5h）

**修改文件**：`src-tauri/tauri.conf.json:27`

| 当前 | 目标 |
|------|------|
| `"csp": null` | `"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:"` |

**注意**：`'unsafe-inline'` 是 Ant Design 样式注入所需，无法避免。`'self'` 禁止加载外部脚本。

**验收**：启动应用，所有页面正常渲染（样式不丢失），无 CSP 报错。

### 任务 S-02：修复 lib.rs 启动路径中的 unwrap/expect（0.5h）

**修改文件**：`src-tauri/src/lib.rs`

| 行号 | 当前代码 | 目标代码 |
|------|---------|---------|
| 25 | `db_path.to_str().unwrap()` | `db_path.to_str().ok_or_else(|| "数据库路径非UTF-8".into())?` — 需改 `run()` 返回类型为 `Result<(), Box<dyn Error>>` |
| 26 | `.expect("无法初始化数据库")` | 直接用 `?`（DbState::new 已返回 Result） |
| 29 | `db_state.pool.get().expect("连接池获取失败")` | `db_state.pool.get()?` — 利用已有的 `From<r2d2::Error>` |
| 35 | `.expect("无法初始化日志系统")` | `?` — logging::init 需改返回类型为 `Result<(), Box<dyn Error>>` |
| 75 | `.expect("error while running RackViz")` | 改为 `unwrap_or_else(|e| { log::error!("运行失败: {}", e); e })` — 主线程 panic 无法避免，至少记录日志 |

**变更模式**：`run()` 函数签名改为 `pub fn run() -> Result<(), Box<dyn std::error::Error>>`，Tauri Builder 用 `.setup()` 中所有 `?` 替换 `.expect()`。

**验收**：`rg '.expect\(' src-tauri/src/lib.rs` 仅剩主线程 `.run()` 一处，`cargo check` 通过。

### 任务 S-03：修复 logging.rs Mutex unwrap（0.5h）

**修改文件**：`src-tauri/src/logging.rs`

| 行号 | 当前 | 目标 |
|------|------|------|
| 45 | `*LOGGER_HANDLE.lock().unwrap() = Some(handle)` | `*LOGGER_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle)` — poisoning 时取原始数据 |
| 57 | 同上 | 同上 |
| 77 | `let mut guard = LOGGER_HANDLE.lock().unwrap()` | 同上模式 |
| 87 | 同上 | 同上 |

**验收**：`rg '\.lock\(\)\.unwrap\(\)' src-tauri/src/logging.rs` 无结果。

### 任务 S-04：修复 db 层 insert 后 unwrap（1h）

**修改文件**（4 个）：

| 文件 | 行号 | 当前 | 目标 |
|------|------|------|------|
| `db/devices.rs:118` | `get_device(conn, id).map(|r| r.unwrap())` | `get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))?` |
| `db/racks.rs:57` | `get_rack(conn, id).map(|r| r.unwrap())` | `get_rack(conn, id)?.ok_or_else(|| AppError::not_found("机柜"))?` |
| `db/rooms.rs:45` | `get_room(conn, id).map(|r| r.unwrap())` | `get_room(conn, id)?.ok_or_else(|| AppError::not_found("机房"))?` |
| `db/device_models.rs:51` | `get_device_model(conn, id).map(|r| r.unwrap())` | `get_device_model(conn, id)?.ok_or_else(|| AppError::not_found("型号"))?` |

**验收**：`rg '\.unwrap\(\)' src-tauri/src/db/ --glob '!*test*'` 无结果（排除测试代码）。

### 任务 S-05：添加数据库索引（迁移 v3）（1h）

**修改文件**：`src-tauri/src/migration.rs`

新增迁移函数 `migrate_v2_to_v3()`：

```sql
-- 7 个索引
CREATE INDEX IF NOT EXISTS idx_devices_rack_id ON devices(rack_id);
CREATE INDEX IF NOT EXISTS idx_devices_serial_no ON devices(serial_no) WHERE serial_no != '';
CREATE INDEX IF NOT EXISTS idx_devices_asset_no ON devices(asset_no) WHERE asset_no != '';
CREATE INDEX IF NOT EXISTS idx_devices_name_rack_id ON devices(name, rack_id);
CREATE INDEX IF NOT EXISTS idx_racks_name ON racks(name);
CREATE INDEX IF NOT EXISTS idx_racks_sort_order ON racks(sort_order);
CREATE INDEX IF NOT EXISTS idx_device_models_name ON device_models(name);

-- 4 个 UNIQUE 约束（部分索引，排除空值）
CREATE UNIQUE INDEX IF NOT EXISTS uniq_devices_serial_no ON devices(serial_no) WHERE serial_no != '';
CREATE UNIQUE INDEX IF NOT EXISTS uniq_devices_asset_no ON devices(asset_no) WHERE asset_no != '';
CREATE UNIQUE INDEX IF NOT EXISTS uniq_racks_name ON racks(name);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_device_models_name ON device_models(name);

-- 外键索引
CREATE INDEX IF NOT EXISTS idx_devices_device_model_id ON devices(device_model_id);
CREATE INDEX IF NOT EXISTS idx_racks_room_id ON racks(room_id);

-- settings 表添加时间戳
ALTER TABLE settings ADD COLUMN created_at TEXT DEFAULT (datetime('now'));
ALTER TABLE settings ADD COLUMN updated_at TEXT DEFAULT (datetime('now'));

PRAGMA user_version = 3;
```

**同时修改**：
- `CURRENT_VERSION` 从 2 改为 3
- `run()` 函数添加 `2 => { migrate_v2_to_v3(conn)?; }`
- 每个 migrate_* 函数添加事务包裹

**验收**：`EXPLAIN QUERY PLAN SELECT * FROM devices WHERE rack_id = 1` 显示 `SEARCH devices USING INDEX idx_devices_rack_id`。

### 任务 S-06：迁移脚本添加事务保护（0.5h）

**修改文件**：`src-tauri/src/migration.rs`

每个迁移函数开头/结尾添加事务：

| 函数 | 修改 |
|------|------|
| `migrate_v0_to_v1()` | 开头 `conn.execute_batch("BEGIN IMMEDIATE")?;`，成功时 `conn.execute_batch("COMMIT")?;` |
| `migrate_v1_to_v2()` | 同上 |
| `migrate_v2_to_v3()` | 同上 |
| `run()` | 整体包裹在事务中，失败时回滚 |

**同时修复**：`run()` 中的 `.unwrap_or(0)` 和 `.unwrap_or(false)` 改为显式错误处理。

**验收**：模拟迁移中途失败（如 v3 迁移中插入错误 SQL），数据库版本号未更新，数据保持原状。

### 任务 S-07：修复 COALESCE 置空 Bug（2h）⚠️ 最复杂

**修改文件**（4 个 db 文件 + 前端适配）：

**方案**：sentinel 值 + 显式字段列表

后端 update 改为显式字段列表模式，前端传 sentinel 值表示"置空"：

| 字段 | sentinel 值 | 置空含义 |
|------|------------|----------|
| `rack_id` | `-1` | 从机柜中移出 |
| `start_u` / `end_u` | `-1` | 清空 U 位 |
| `device_model_id` | `-1` | 清空型号关联 |
| `room_id`（racks） | `-1` | 清空机房关联 |

**`db/devices.rs:130-167` 修改为**：

```rust
pub fn update_device(conn: &Connection, id: i32, data: &DeviceUpdate) -> Result<Option<Device>, AppError> {
    let existing = get_device(conn, id)?;
    if existing.is_none() { return Ok(None); }
    
    let purchase_date = parse_optional_date(&data.purchase_date);
    let warranty_expire = parse_optional_date(&data.warranty_expire);
    
    // sentinel 值转换：-1 → NULL
    let rack_id: Option<i32> = match data.rack_id {
        Some(-1) => None,   // 置空
        Some(v) => Some(v), // 更新为新值
        None => existing.unwrap().rack_id,  // 保持不变
    };
    let start_u: Option<i32> = match data.start_u {
        Some(-1) => None,
        Some(v) => Some(v),
        None => existing.unwrap().start_u,
    };
    let end_u: Option<i32> = match data.end_u {
        Some(-1) => None,
        Some(v) => Some(v),
        None => existing.unwrap().end_u,
    };
    let device_model_id: Option<i32> = match data.device_model_id {
        Some(-1) => None,
        Some(v) => Some(v),
        None => existing.unwrap().device_model_id,
    };
    
    conn.execute(
        "UPDATE devices SET
            name = ?1, device_model_id = ?2, rack_id = ?3,
            start_u = ?4, end_u = ?5, ip_addresses = ?6,
            serial_no = ?7, asset_no = ?8, department = ?9,
            owner = ?10, function = ?11, purchase_date = ?12,
            warranty_expire = ?13, status = ?14, power_watt = ?15
        WHERE id = ?16",
        params![data.name, device_model_id, rack_id, start_u, end_u,
                data.ip_addresses, data.serial_no, data.asset_no,
                data.department, data.owner, data.function,
                purchase_date, warranty_expire, data.status, data.power_watt, id],
    )?;
    get_device(conn, id)
}
```

**同理修改**：`db/racks.rs:66-73`、`db/rooms.rs:53-54`、`db/device_models.rs:60-65`

**前端适配**：`RackView.tsx` 拖拽到资源池时传 `rack_id: -1`（而非 `null`）。

**验收**：
1. 拖拽设备到资源池 → `rack_id` 变为 `NULL` → 设备从机柜消失 ✅
2. 正常更新设备 → 字段正常修改 ✅
3. 清空 `serial_no` → 传空字符串而非 NULL ✅

### 任务 S-08：日志敏感信息脱敏（0.3h）

**修改文件**：`src-tauri/src/excel.rs`

| 行号 | 当前 | 目标 |
|------|------|------|
| 276 | `log::warn!("导入跳过第{}行: 序列号 {} 已被设备「{}」使用", row_idx+1, serial_no, existing.name)` | `log::warn!("导入跳过第{}行: 序列号重复，已被设备「{}」(id:{})使用", row_idx+1, existing.name, existing.id)` |
| 285 | `log::warn!("导入跳过第{}行: 资产编号 {} ...", row_idx+1, asset_no, existing.name)` | `log::warn!("导入跳过第{}行: 资产编号重复，已被设备「{}」(id:{})使用", row_idx+1, existing.name, existing.id)` |

**验收**：`rg 'serial_no\|asset_no\|ip_addresses' src-tauri/src/ --glob '*log*'` 日志语句中无敏感字段明文。

### 任务 S-09：LIKE 通配符转义（0.5h）

**修改文件**：`src-tauri/src/db/devices.rs`

新增辅助函数：

```rust
/// 义 LIKE 通配符 %, _, \
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}
```

修改 `list_devices()` 中搜索逻辑（行 37-38）：

```rust
// 当前：param_values.push(format!("%{}%", s));
// 目标：
let escaped = escape_like(s.trim());
conditions.push(format!("name LIKE ?{} ESCAPE '\\'", param_values.len() + 1));
param_values.push(format!("%{}%", escaped));
```

**验收**：搜索 `100%` 只匹配含 `100%` 的名称，不匹配 `1000`。

### 任务 S-10：核心写操作添加 try/catch（1h）

**修改文件**：`frontend/src/pages/RackView.tsx`

| 行号 | 当前 | 目标 |
|------|------|------|
| 288-295 | `await update(device.id, data)` | `try { await update(...) } catch(e) { message.error('操作失败: ' + e.message) }` |
| 306-317 | `await handleStockDrop` | 同上 |
| 319-338 | `await createRack(...)` | 同上 |
| 372-384 | `await update(id, data)` 机柜移动 | 同上 |

**同理修改**：`DeviceList.tsx:399-404`（导出操作添加 try/catch）

**验收**：模拟后端返回错误，前端显示 `message.error` 提示。

### 任务 S-11：设置操作错误对用户可见（0.3h）

**修改文件**：`frontend/src/components/Layout.tsx`

| 行号 | 当前 | 目标 |
|------|------|------|
| 71-74 | `.catch(console.error)` | `.catch(err => message.error('获取日志配置失败'))` |
| 85 | `console.error('切换日志失败:', err)` | `message.error('切换日志失败')` |
| 95 | `console.error('打开日志目录失败:', err)` | `message.error('打开日志目录失败')` |

**同理修改**：`useApiList.ts:34` — `.catch(console.error)` → `.catch(err => { message.error('数据加载失败'); console.error(err); })`

**验收**：模拟设置请求失败，用户可见错误提示。

### 任务 S-12：统一类型定义源（2h）⚠️ 影响面广

**修改文件**：`frontend/src/tauri-api.ts` + `frontend/src/types/index.ts`

**方案**：
1. `types/index.ts` 保持为**唯一类型定义源**
2. `tauri-api.ts` 删除所有 `*Resp`/`*Create`/`*Update` 类型定义，改为 `import type { Device, Room, ... } from '../types'`
3. `tauri-api.ts` 添加 API 响应→领域模型的转换函数

**types/index.ts 增强**（补充缺失的 Create/Update 类型）：

```typescript
// 已有：Device, DeviceModel, Rack, Room, DeviceType
// 新增：
export interface DeviceCreate { ... }
export interface DeviceUpdate { ... }
export interface RackCreate { ... }
export interface RackUpdate { ... }
export interface RoomCreate { ... }
export interface RoomUpdate { ... }
export interface ModelCreate { ... }
export interface ModelUpdate { ... }
```

**tauri-api.ts 改为**：

```typescript
import type { Device, Room, Rack, DeviceModel, ... } from '../types';
// API 函数签名使用 types/index.ts 的类型
// invoke 返回值用转换函数处理 status/view/type 等枚举字段
```

**消除的 as 断言**：4 个 hooks 中的 13 处 `as Room[]`/`as Device[]` 等，改为直接使用统一类型。

**验收**：`rg 'interface Device\|interface Room\|interface Rack\|interface DeviceModel' frontend/src/` 每个类型只定义一次。

### 任务 S-13：AppError 实现 Error trait（0.3h）

**修改文件**：`src-tauri/src/error.rs`

添加：

```rust
impl std::error::Error for AppError {}
```

**验收**：`AppError` 可作为 `dyn std::error::Error` 使用，`?` 运算符正常工作。

### 任务 S-14：ROLLBACK 失败记录日志（0.3h）

**修改文件**：`src-tauri/src/state.rs:71`

```rust
// 当前：let _ = conn.execute_batch("ROLLBACK");
// 目标：
if let Err(e) = conn.execute_batch("ROLLBACK") {
    log::error!("[异常] ROLLBACK 失败: {}", e);
}
```

**验收**：ROLLBACK 失败时日志有记录。

---

## 四、Phase 2：后端健壮性（3 天，P1 后端）

### 任务 B-01：连接池错误分类修复（1h）⚠️ 27 处

**修改文件**：所有 `commands/*.rs`

**当前模式**（所有 6 个文件）：
```rust
let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
```

**目标模式**：
```rust
let conn = state.pool.get()?;  // 利用已有的 From<r2d2::Error> for AppError，自动分类为 DatabaseError
```

**涉及文件**：
- `commands/devices.rs:13,19,31,39,51`
- `commands/racks.rs:9,15,24,32,42`
- `commands/rooms.rs:9,15,24,32,42`
- `commands/device_models.rs:9,15,24,32,42`
- `commands/settings.rs:15,28`
- `commands/exports.rs:21,41,62,82,102`

**验收**：`rg 'map_err\(AppError::io\)' src-tauri/src/commands/` 无结果。

### 任务 B-02：封装连接池获取方法（0.5h）

**修改文件**：`src-tauri/src/state.rs`

新增：

```rust
impl DbState {
    /// 获取数据库连接 — 封装 pool.get()，统一错误处理
    pub fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, AppError> {
        self.pool.get().map_err(AppError::from)
    }
}
```

**commands/*.rs 简化为**：
```rust
let conn = state.conn()?;  // 替代 state.pool.get().map_err(...)
```

**验收**：27 处 `state.pool.get().map_err(...)` 简化为 `state.conn()?`。

### 任务 B-03：.ok() 吞错误修复（1h）

**修改文件**：`db/devices.rs`, `db/settings.rs`, `commands/*.rs`

**db 层修复**：

| 文件 | 行号 | 当前 | 目标 |
|------|------|------|------|
| `db/settings.rs:9` | `Ok(conn.query_row(...).ok())` | 使用 `rusqlite::OptionalExtension`：`Ok(conn.query_row(...).optional()?.)` |
| `db/devices.rs:201` | `stmt.query_row(...).ok()` | `.optional()?.map(|d| d)` |
| `db/devices.rs:210` | 同上 | 同上 |

**command 层修复**（delete 前查名称）：

```rust
// 当前：db::devices::get_device(&conn, id).ok().flatten().map(|d| d.name).unwrap_or_default()
// 目标：匹配错误类型
let name = match db::devices::get_device(&conn, id) {
    Ok(Some(d)) => d.name,
    Ok(None) => String::new(),  // 设备不存在
    Err(e) => { log::error!("查询设备名称失败: {}", e); String::new() }
};
```

**验收**：`rg '\.ok\(\)' src-tauri/src/db/ src-tauri/src/commands/` 无 .ok() 调用（排除测试代码和 OptionalExtension）。

### 任务 B-04：导出命令异步化（1h）

**修改文件**：`commands/exports.rs`

5 个导出命令改为 async + `spawn_blocking`：

```rust
#[tauri::command]
pub async fn export_racks_excel(state: State<'_, DbState>) -> Result<String, AppError> {
    let pool = state.pool.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = pool.get()?;
        // ... 原同步逻辑
    }).await.map_err(|e| AppError::io(&format!("导出任务失败: {}", e)))?
}
```

**验收**：导出 100 机柜 Excel 时 UI 不冻结。

### 任务 B-05：修复 N+1 查询（0.5h）

**修改文件**：`src-tauri/src/excel.rs:157-169`

**当前**：逐 rack 查 devices（N+1）

**目标**：一次查全部 devices + 内存分组

```rust
// 一次查询所有设备
let all_devices = db::devices::list_devices(&c, None, None)?;
// 内存按 rack_id 分组
let devices_by_rack: HashMap<i32, Vec<&Device>> = all_devices.iter()
    .filter_map(|d| d.rack_id.map(|rid| (rid, d)))
    .into_group_map();  // 或手动 collect 到 HashMap
```

**验收**：导出 100 机柜时只有 2 次 SQL 查询（1 次查 racks + 1 次查全部 devices）。

### 任务 B-06：format!() SQL 模式重构（0.5h）

**修改文件**：`src-tauri/src/db/devices.rs:27-57`

**方案**：将动态条件查询改为静态 SQL + 分支：

```rust
pub fn list_devices(conn: &Connection, rack_id: Option<i32>, search: Option<String>) -> Result<Vec<Device>, AppError> {
    let escaped_search = search.map(|s| escape_like(s.trim()));
    
    let sql;
    let params: Vec<Box<dyn rusqlite::types::ToSql>>;
    
    match (rack_id, &escaped_search) {
        (None, None) => {
            sql = "SELECT ... FROM devices ORDER BY name";
            params = vec![];
        }
        (Some(rid), None) => {
            sql = "SELECT ... FROM devices WHERE rack_id = ?1 ORDER BY name";
            params = vec![Box::new(rid)];
        }
        (None, Some(s)) => {
            sql = "SELECT ... FROM devices WHERE name LIKE ?1 ESCAPE '\\' ORDER BY name";
            params = vec![Box::new(format!("%{}%", s))];
        }
        (Some(rid), Some(s)) => {
            sql = "SELECT ... FROM devices WHERE rack_id = ?1 AND name LIKE ?2 ESCAPE '\\' ORDER BY name";
            params = vec![Box::new(rid), Box::new(format!("%{}%", s))];
        }
    }
    // ...
}
```

**验收**：`rg 'format!\(' src-tauri/src/db/devices.rs | grep -i 'select\|where\|from'` 无 format! 拼接 SQL 字符串。

### 任务 B-07：SELECT 列列表提取为常量（0.5h）

**修改文件**：`db/devices.rs`, `db/racks.rs`, `db/rooms.rs`, `db/device_models.rs`

每个模块提取列常量：

```rust
const DEVICE_COLUMNS: &str = "id, name, device_model_id, rack_id, start_u, end_u, ip_addresses, serial_no, asset_no, department, owner, function, purchase_date, warranty_expire, status, power_watt";
```

所有 SQL 中的列列表改为引用常量。

**验收**：每个实体类型列列表只定义一次。

### 任务 B-08：find_or_create 竞态修复（1h）

**修改文件**：`db/racks.rs:101-112`, `db/device_models.rs:87-98`

改为 `INSERT ... ON CONFLICT DO NOTHING RETURNING id` 或 `INSERT OR IGNORE` + 查询：

```rust
pub fn find_or_create_rack(conn: &Connection, name: &str, ...) -> Result<i32, AppError> {
    // 先尝试查询
    if let Some(id) = find_rack_by_name(conn, name)? {
        return Ok(id);
    }
    // INSERT OR IGNORE 避免竞态
    conn.execute("INSERT OR IGNORE INTO racks (name, ...) VALUES (?1, ...)", params![name, ...])?;
    // 再查询（无论 insert 还是 ignore 都能拿到 id）
    find_rack_by_name(conn, name)?.ok_or_else(|| AppError::not_found("机柜"))
}
```

**前提**：任务 S-05 中 UNIQUE 约束已添加。

**验收**：并发创建同名机柜/型号不产生重复记录。

### 任务 B-09：启用外键约束持久化（0.3h）

**修改文件**：`src-tauri/src/state.rs:18-20`

**当前**：`PRAGMA foreign_keys=ON` 在 `RackVizConnectionCustomizer::on_acquire()` 中，每次连接获取时执行。

**检查**：该 PRAGMA 是否被持久化。SQLite 的 `foreign_keys` PRAGMA 不被持久化，需每次连接设置。当前实现已正确。

**无需修改**，仅需验收确认。

**验收**：删除被引用的机房时返回友好错误（而非静默成功）。

### 任务 B-10：row_to_* 映射函数统一（0.5h）

**修改文件**：提取 `row_to_device`/`row_to_rack`/`row_to_room`/`row_to_device_model` 为公共函数。

当前每个 db 模块有自己的 `row_to_*` 函数，但逻辑重复（尤其是 devices 的 16 列映射）。提取到 `db/mod.rs` 或独立 `db/mappers.rs`。

**验收**：每个实体的 row_to_* 函数只实现一次。

---

## 五、Phase 3：前端重构（4 天，P1 前端）

### 任务 F-01：拆分 RackView 组件（4h）⚠️ 最大任务

**当前**：`RackView.tsx` 879 行，13 个 useState

**拆分方案**：

```
pages/RackView.tsx (主页面，< 200 行)
├── components/RackGrid.tsx (机柜网格渲染，~150 行)
├── components/DeviceStockPanel.tsx (资源池面板，~100 行)
├── components/AddRackModal.tsx (新增机柜弹窗，~80 行)
├── components/EditDeviceModal.tsx (编辑设备弹窗，~100 行)
├── components/EditRackModal.tsx (编辑机柜弹窗，~80 行)
└── hooks/useRackViewState.ts (状态管理 hook，~80 行)
```

**拆分步骤**：
1. 先提取 `useRackViewState` hook（合并 13 个 useState 为 useReducer 或分组）
2. 提取 AddRackModal / EditDeviceModal / EditRackModal
3. 提取 RackGrid / DeviceStockPanel
4. RackView 主页面仅负责布局和状态协调

**验收**：每个文件 < 300 行，每个组件 useState < 8 个。

### 任务 F-02：拆分 DeviceList 组件（2h）

**当前**：`DeviceList.tsx` 577 行，11 个 useState

**拆分方案**：

```
pages/DeviceList.tsx (主页面，< 200 行)
├── components/DeviceTable.tsx (设备表格，~150 行)
├── components/DeviceImportModal.tsx (导入弹窗，~80 行)
├── components/ResizableTitle.tsx (列宽拖拽标题，~50 行)
└── hooks/useDeviceListState.ts (状态管理 hook，~80 行)
```

**验收**：每个文件 < 300 行，useState < 8 个。

### 任务 F-03：拆分 LayoutContext（2h）

**当前**：`Layout.tsx` 252 行，12 个 useState，23 个 Context 字段

**拆分方案**：

```typescript
// 拆分为两个 Context
contexts/ViewContext.tsx   // 视图控制：selectedRackId, selectedDeviceId, searchVisible 等 (~10 字段)
contexts/RoomContext.tsx   // 机房管理：rooms, currentRoomId, addRoom, editRoom 等 (~10 字段)
```

**验收**：每个 Context < 10 字段，每个文件 useState < 8 个。

### 任务 F-04：移除 as 类型断言（2h）

**前提**：任务 S-12（类型统一）完成后执行。

**修改文件**：4 个 hooks + useApiList.ts

**当前**：13 处 `as Room[]`/`as Device[]` 等

**目标**：hooks 直接使用统一类型，无需断言。

| 文件 | 断言数 | 目标 |
|------|:---:|------|
| `useRooms.ts` | 3 | 直接 `api.listRooms()` 返回 `Room[]` |
| `useRacks.ts` | 4 | 同理 |
| `useDevices.ts` | 3 | 同理 |
| `useDeviceModels.ts` | 3 | 同理 |
| `useApiList.ts:51` | 1 | 改为运行时校验函数 |

**DeviceList.tsx 中 4 处 as 断言**：
- `col.key as DeviceColumnKey` → 用类型守卫或直接定义列 key 为 DeviceColumnKey 类型

**验收**：`rg 'as [A-Z]\w+\[\]\|as Device\|as Room\|as Rack\|as DeviceModel' frontend/src/` 无结果。

### 任务 F-05：列表过滤/排序添加 useMemo（0.5h）

**修改文件**：

| 文件 | 行号 | 修改 |
|------|------|------|
| `DeviceList.tsx:295` | `const filteredDevices = devices.filter(...)` | `const filteredDevices = useMemo(() => devices.filter(...), [devices, searchText, roomRackIds])` |
| `DeviceList.tsx:302` | `const filteredModels = models.filter(...)` | `const filteredModels = useMemo(() => models.filter(...), [models, modelSearchText])` |
| `RoomTabs.tsx:103` | `const sortedRooms = [...rooms].sort(...)` | `const sortedRooms = useMemo(() => [...rooms].sort(...), [rooms])` |

**验收**：React DevTools Profiler 显示交互时不再重算过滤结果。

### 任务 F-06：列定义提取到 useMemo 或组件外（0.5h）

**修改文件**：`DeviceList.tsx:306-368`

```typescript
// 提取到组件外或 useMemo
const allDeviceColumns = useMemo(() => [...], [models, racks, rooms]);
const visibleDeviceColumns = useMemo(() => ..., [allDeviceColumns, visibleColumns, columnWidths]);
const modelColumns = useMemo(() => [...], [models]);
```

**验收**：列定义不在每次渲染时重建新对象。

### 任务 F-07：! 非空断言修复（0.5h）

**修改文件**：

| 文件 | 行号 | 修改 |
|------|------|------|
| `RackView.tsx:517` | `device.end_u! - device.start_u!` | `(device.end_u ?? 0) - (device.start_u ?? 0)` 或前置判空 |
| `main.tsx:99` | `document.getElementById('root')!` | `document.getElementById('root') ?? document.createElement('div')` 或前置断言 |

**验收**：`rg '!\\.' frontend/src/ | grep -v 'node_modules' | grep -v '.d.ts'` 无非空断言。

### 任务 F-08：useCallback 依赖稳定化（1h）

**修改文件**：`RackView.tsx`

| 函数 | 问题 | 目标 |
|------|------|------|
| `getDevicesForRack` (行 132) | 每次 `devices.filter()` | 用 `useMemo` 生成 `devicesByRackId: Map<i32, Device[]>`，`getDevicesForRack` 改为 Map.get |
| `getDeviceModel` (行 136) | 每次 `models.find()` | 用 `useMemo` 生成 `modelsById: Map` |
| `getRoomName` (行 137) | 每次 `rooms.find()` | 用 `useMemo` 生成 `roomsById: Map` |

**验收**：`useCallback` 缓存命中，React DevTools 不显示不必要的重渲染。

### 任务 F-09：ctxValue useMemo 依赖补全（0.3h）

**修改文件**：`Layout.tsx:140-155`

补全缺失的 5 个函数引用到 `ctxValue` 的 useMemo 依赖列表：
- `handleAddRoom`, `handleEditRoom`, `handleDeleteRoom`, `handleUpdateRoom`, `handleSetCurrentRoomId`

**验收**：ESLint `react-hooks/exhaustive-deps` 规则不报依赖缺失警告。

### 任务 F-10：移除 any 类型（0.5h）

**修改文件**：`useApiList.ts:3-4`

```typescript
// 当前：
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyRecord = Record<string, any>;  // 死代码

// 目标：删除整行（AnyRecord 从未被使用）
```

**验收**：`rg ': any\b' frontend/src/` 无 any 类型。

### 任务 F-11：状态标签/工具函数去重（1h）

**当前**：`getStatusText`/`getStatusTag` 在 `DeviceList.tsx`、`RackView.tsx`、`DeviceDetailPanel.tsx` 各实现一次

**方案**：提取为 `utils/status.ts`

```typescript
// frontend/src/utils/status.ts
export function getStatusText(status: string): string { ... }
export function getStatusTag(status: string): { color: string; label: string } { ... }
export function getDeviceTypeLabel(type: string): string { ... }
```

**验收**：每个工具函数只实现一次。

---

## 六、Phase 4：功能增强（8 天，可选）

> Phase 4 为功能增强，可与 Phase 3 并行开发，但不阻塞核心升级。建议核心 Phase 0-3 完成后再执行。

### N-01：分页查询支持（1d）

**后端**：`list_devices` 添加 `offset: i32, limit: i32` 参数

**前端**：DeviceList 添加 Ant Design Pagination 组件

### N-02：多字段搜索（0.5d）

**后端**：`list_devices` 支持 `search_field` 参数（name/ip_addresses/serial_no/asset_no）

**前端**：搜索框下拉选择搜索字段

### N-03：导入更新模式（1d）

**后端**：`import_devices_excel` 支持 `on_conflict: "skip" | "update"` 参数

**前端**：导入对话框添加"跳过重复/覆盖更新"选项

### N-04：操作撤销 Ctrl+Z（2d）

**前端**：实现 `UndoStack<T>` 操作历史栈，支持撤销/重做

### N-05：键盘快捷键（1d）

**前端**：使用已引入的 `tauri-plugin-global-shortcut`，添加 Ctrl+N/F/Z/Delete/F5

### N-06~N-10：其他增强

时间戳字段、搜索结果高亮、拖拽确认提示、暗色主题优化、导出 Excel 增强 — 每项 0.5d-1d

---

## 七、Phase 5：质量基建（3 天）

### 任务 Q-01：代码去重（2h）

执行 Phase 2 中 B-06~B-10 的所有任务（列常量、row_to_*、日期解析、连接池封装、测试 setup）。

### 任务 Q-02：输入验证增强（1h）

**后端**：添加 `start_u <= end_u`、IP 格式、枚举值验证

**前端**：Ant Design Form 规则增强

### 任务 Q-03：CI 配置（1h）

**创建文件**：`.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]
jobs:
  rust-check:
    runs-on: ubuntu-latest
    steps: [checkout, cargo check, cargo clippy, cargo test]
  frontend-check:
    runs-on: ubuntu-latest
    steps: [checkout, npm install, npx tsc --noEmit, npm run lint]
```

### 任务 Q-04：版本号验证脚本（0.3h）

**创建文件**：`scripts/check-version.sh`

检查 Cargo.toml / tauri.conf.json / package.json / Layout.tsx 四处版本一致。

---

## 八、验收标准

### 8.1 代码红线验收（自动化检查命令）

| 类别 | 检查命令 | 期望结果 |
|------|---------|---------|
| Rust 无 unwrap | `rg '\.unwrap\(\)' src-tauri/src/ --glob '!*test*'` | 仅测试中出现 |
| Rust 无 expect | `rg '\.expect\(' src-tauri/src/` | 仅 `.run()` 主线程一处 |
| Rust 无 COALESCE | `rg 'COALESCE' src-tauri/src/db/` | 无结果 |
| Rust 无 map_err(AppError::io) | `rg 'map_err\(AppError::io\)' src-tauri/src/commands/` | 无结果 |
| Rust 无 .ok() 吞错 | `rg '\.ok\(\)' src-tauri/src/db/ src-tauri/src/commands/` | 仅 OptionalExtension |
| 前端无 any | `rg ': any\b' frontend/src/` | 无结果 |
| 前端无 as 断言 | `rg 'as [A-Z]\w+' frontend/src/` | 仅 JSON.parse 等不可避免场景 |
| 前端无 ! 非空断言 | `rg '!\\.' frontend/src/**/*.tsx` | 无结果 |
| 前端 invoke 全 try/catch | `rg 'await invoke' frontend/src/tauri-api.ts` | 全在 try 内 |
| 前端组件行数 | `wc -l frontend/src/pages/*.tsx` | 最大 < 300 |
| 前端 useState 数 | 人工审查 | 最大 < 8 |
| 前端 Context 字段 | 人工审查 | 最大 < 10 |

### 8.2 功能验收

| 功能 | 验收方法 | 标准 |
|------|---------|------|
| 拖拽到资源池 | 拖拽设备到资源池 | rack_id 变 NULL，设备从机柜消失 |
| 搜索 100% | 搜索含 % 的字符串 | 只匹配含 % 的设备名 |
| 重复 serial_no | 创建相同序列号设备 | 返回友好错误 |
| 导出大文件 | 导出 100 机柜 Excel | UI 不冻结 |
| 操作失败提示 | 模拟后端错误 | message.error 显示 |
| CSP 保护 | 检查 tauri.conf.json | CSP 非 null |
| 索引覆盖 | EXPLAIN QUERY PLAN | 高频查询使用索引 |

---

## 九、风险与应对

| 风险 | 级别 | 影响 | 应对措施 |
|------|:---:|------|---------|
| COALESCE→sentinel 回归 | 🔴 | 拖拽功能失效 | Phase 1 末手动测试拖拽场景 |
| 迁移 v3 失败 | 🟡 | 旧版数据库无法升级 | 迁移前备份 + 事务回滚 |
| 前端拆分引入 Bug | 🟡 | 功能异常 | 逐组件拆分，每步 cargo check + tsc |
| 类型统一编译错误 | 🟡 | 构建失败 | 先 tsc --noEmit 再 npm build |
| Phase 3 超预期 | 🟡 | 延期 2-3 天 | 优先 RackView 拆分，其余可延后 |

---

## 十、依赖关系与执行顺序

```
Phase 0 ──────────────────────────────────────────────────
  │
  ├─→ Phase 1（P0 全清）
  │     ├─→ S-05（索引+UNIQUE） ← 前置：无
  │     ├─→ S-06（迁移事务） ← 前置：S-05（同迁移 v3）
  │     ├─→ S-07（COALESCE） ← 前置：S-05（UNIQUE 约束支撑 sentinel）
  │     ├─→ S-12（类型统一） ← 前置：无（可独立）
  │     └─→ 其他 S-01~S-14 ← 前置：无
  │
  ├─→ Phase 2（后端 P1）
  │     ├─→ B-01/B-02（连接池） ← 前置：S-13（Error trait）
  │     ├─→ B-08（find_or_create） ← 前置：S-05（UNIQUE 约束）
  │     ├─→ B-06（SQL 重构） ← 前置：S-09（LIKE 转义）
  │     └─→ 其他 ← 前置：Phase 1 完成
  │
  ├─→ Phase 3（前端 P1）
  │     ├─→ F-01~F-03（组件拆分） ← 前置：无
  │     ├─→ F-04（移除 as） ← 前置：S-12（类型统一）
  │     ├─→ F-05~F-08（性能优化） ← 前置：F-01（拆分后优化）
  │     └─→ 其他 ← 前置：Phase 1 完成
  │
  ├─→ Phase 4（功能增强） ← 可与 Phase 3 并行
  │
  └─→ Phase 5（质量基建） ← 前置：Phase 2+3 完成
```

**关键路径**：Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 5 → 最终验收

---

## 十一、最终发布 Checklist

| 任务 | 验收标准 |
|------|---------|
| 代码红线全清 | 所有自动化检查命令通过 |
| 功能回归测试 | 所有 v1.1 功能正常 |
| 性能测试 | 搜索 < 100ms，导出 < 1s，拖拽 < 100ms |
| 安全测试 | CSP 配置正确，日志无敏感信息 |
| 构建 | `cargo tauri build --no-bundle` 成功 |
| 版本号一致 | 四处均为 1.2.0 |
| 文档更新 | 产品介绍、用户手册同步 |

---

*本方案基于 `RackViz-项目全面梳理报告.md` 的实际代码扫描结果，每个任务都附有精确的文件路径和修改目标。建议按 Phase 顺序严格执行，每个 Phase 完成后进行验收测试再进入下一 Phase。*
