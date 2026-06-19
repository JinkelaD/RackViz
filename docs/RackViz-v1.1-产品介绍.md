# RackViz v1.1 — 机房设备可视化管理系统

> **重新定义数据中心资产管理方式**  
> 从入库上架到退役报废，全生命周期可视化管理

---

## 一、产品概述

RackViz 是一款面向数据中心运维人员的**桌面级机房设备可视化管理系统**。它以机柜为可视化核心，通过直观的拖拽交互完成设备上下架操作，同时提供完整的设备台账管理、数据导入导出、报表生成和运维日志审计能力。

### 核心价值

| 痛点 | RackViz 解决方案 |
|------|-----------------|
| 机柜设备位置靠 Excel 记录，不直观 | 可视化机柜网格，U 位精确到行 |
| 设备上下架需要改表改文档 | 拖拽即上架，一键即下架 |
| 多人运维缺少操作记录 | 操作审计日志，可追溯 |
| 批量录入设备费时费力 | Excel 一键导入，自动查重 |
| 数据散落各处，报表靠手动 | Excel/HTML 报表一键生成 |

---

## 二、功能全景

### 🖥️ 机柜可视化

| 功能 | 说明 |
|------|------|
| **机柜网格视图** | 以标准 42U 机柜为模板，逐行渲染 U 位，设备块颜色编码区分类型 |
| **正/背面视图** | 支持机柜正面和背面视图切换 |
| **缩放控制** | 50%-200% 无级缩放，点击百分比重置 |
| **U 位使用率** | 每个机柜底部显示使用率进度条，超过 85% 红色预警 |
| **设备悬浮提示** | 鼠标悬停设备显示名称、型号、类型、U位、状态 |
| **设备状态指示** | 开机=绿色脉冲动画，离线=灰色，未上架=虚线边框 |

### 🔄 拖拽上下架

| 功能 | 说明 |
|------|------|
| **资源池拖入机柜** | 从侧边栏资源池拖设备到机柜，自动寻找最近可用 U 位 |
| **机柜间迁移** | 直接拖拽设备到另一机柜，自动放置到最优位置 |
| **机柜拖回资源池** | 拖到侧边栏即可下架，设备状态自动变更为「未上架」 |
| **U 位冲突检测** | 拖入时检测空间是否充足，冲突时给出提示 |
| **放置预览** | 拖拽过程中目标位置高亮显示 |

### 📋 设备台账

| 功能 | 说明 |
|------|------|
| **全字段表格** | 11 列可配置：名称、型号、机房、机柜、位置、IP、资产编号、部门、责任人、状态、操作 |
| **列显示控制** | Popover 弹窗勾选显示/隐藏列 |
| **列宽拖拽** | 拖拽表头边缘调整列宽 |
| **多列排序** | 每列支持中文排序 |
| **分页浏览** | 10/20/50 条每页切换 |
| **按机房过滤** | 通过底部机房标签页筛选 |
| **名称搜索** | 实时模糊搜索 |
| **一键刷新** | 刷新按钮手动更新数据 |

### 🏢 机房管理

| 功能 | 说明 |
|------|------|
| **CRUD 操作** | 新建/编辑/删除机房 |
| **标签页导航** | 底部标签页切换机房过滤 |
| **拖拽排序** | 拖动机房标签调整显示顺序 |
| **内联编辑** | 双击标签即可修改机房名称 |

### 🗄️ 机柜管理

| 功能 | 说明 |
|------|------|
| **创建机柜** | 可自定义名称、高度(4-48U)、所属机房 |
| **编辑机柜** | 双击机柜头部打开编辑弹窗 |
| **删除机柜** | 删除后设备保留，自动变为「未上架」 |
| **左右排序** | 机柜头部排序按钮调整位置 |

### 🔧 设备型号管理

| 功能 | 说明 |
|------|------|
| **型号 CRUD** | 新建/编辑/删除设备型号 |
| **类型标签** | 5 种设备类型彩色标签：服务器/交换机/路由器/存储/安全设备 |
| **U 高度定义** | 型号关联高度，创建设备时自动计算结束 U 位 |
| **型号搜索** | 型号管理弹窗内实时搜索 |

### 📥📥 数据导入导出

| 功能 | 说明 |
|------|------|
| **Excel 导入** | 原生文件对话框选择 xlsx，15 列完整映射 |
| **三重查重** | 导入时自动检测：序列号唯一 → 资产编号唯一 → 同机柜名称唯一 |
| **自动创建关联** | 导入时型号/机柜不存在则自动创建 |
| **导入结果反馈** | 显示「成功 X 条，跳过重复 Y 条」 |
| **设备台账导出** | Excel 格式，15 列完整数据 |
| **机柜部署图导出** | 全部机柜横向排列，U 位逐行填充设备名 |
| **单机柜导出** | 单个机柜的部署图 |
| **HTML 报表** | 设备报表含统计摘要（总设备/开机/离线/未上架数量） |

### 🌓 主题系统

| 功能 | 说明 |
|------|------|
| **Dark / Light 切换** | 一键切换，含 Ant Design 全组件主题适配 |
| **主题持久化** | 偏好存储到 localStorage，重启保持 |
| **全局覆盖** | 表格、弹窗、按钮、输入框等全覆盖 |

### ⚙️ 日志与设置

| 功能 | 说明 |
|------|------|
| **日志收集开关** | 设置弹窗中一键开启/关闭 |
| **运行时切换** | 无需重启，立即生效 |
| **日志文件轮转** | 按天切割，自动保留最近 7 天 |
| **打开日志目录** | 一键跳转到日志文件夹 |
| **操作审计日志** | 所有 CRUD 和导入导出操作自动记录 |

---

## 三、技术架构

### 架构图

```
┌─────────────────────────────────────────────────┐
│              RackViz 桌面应用                      │
│                                                   │
│  ┌─────────────────┐    ┌──────────────────────┐ │
│  │   前端 (WebView)  │    │   后端 (Rust/Tauri)    │ │
│  │                   │    │                        │ │
│  │  React 18        │    │  Tauri 2.x Runtime     │ │
│  │  Ant Design 5    │◄──►│  r2d2 连接池 (4连接)    │ │
│  │  Vite 6          │IPC│  SQLite (WAL模式)       │ │
│  │  TypeScript 5    │    │  flexi_logger          │ │
│  │                   │    │  calamine/rust_xlsxwriter│ │
│  └─────────────────┘    └──────────────────────┘ │
│                                                   │
│              ┌─────────────┐                      │
│              │  rackviz.db  │  SQLite 数据库        │
│              └─────────────┘                      │
└─────────────────────────────────────────────────┘
```

### 技术栈详情

| 层级 | 技术 | 版本 | 用途 |
|------|------|------|------|
| **桌面框架** | Tauri | 2.x | 跨平台桌面应用，替代 Electron |
| **后端语言** | Rust | 2021 Edition | 高性能安全后端 |
| **前端框架** | React | 18.3 | UI 渲染 |
| **UI 组件库** | Ant Design | 5.22 | 企业级组件 |
| **构建工具** | Vite | 6.4 | 前端构建与 HMR |
| **类型系统** | TypeScript | 5.6 | 类型安全 |
| **数据库** | SQLite | 3.x (bundled) | 嵌入式关系数据库 |
| **连接池** | r2d2 + r2d2_sqlite | 0.8 / 0.25 | 并发安全数据库访问 |
| **Excel 读取** | calamine | 0.25 | xlsx 解析 |
| **Excel 写入** | rust_xlsxwriter | 0.79 | xlsx 生成 |
| **HTML 报表** | Askama | 0.12 | 编译期模板引擎 |
| **日期处理** | chrono | 0.4 | 日期解析与格式化 |
| **日志系统** | flexi_logger + log | 0.29 / 0.4 | 可配置日志与运行时切换 |
| **路由** | React Router | 6.28 | SPA 路由管理 |
| **字体** | Space Grotesk + JetBrains Mono | — | UI 字体 + 等宽字体 |

### 后端架构

```
src-tauri/src/
├── main.rs              # 程序入口，release 模式隐藏控制台
├── lib.rs               # Tauri Builder 配置，24 个命令注册
├── models.rs            # 13 个数据结构 (4 实体 + 4 Create + 4 Update + ImportResult)
├── error.rs             # 统一错误类型 (7 种错误码)
├── state.rs             # r2d2 连接池 + 事务包装器
├── migration.rs         # 数据库版本迁移 (v0→v1→v2)
├── logging.rs           # flexi_logger 动态日志
├── excel.rs             # Excel 导入导出核心逻辑
├── report.rs            # HTML 报表生成
├── commands/            # Tauri IPC 命令层 (参数校验 + 操作日志)
│   ├── rooms.rs         # 5 个机房命令
│   ├── racks.rs         # 5 个机柜命令
│   ├── devices.rs       # 5 个设备命令
│   ├── device_models.rs # 5 个型号命令
│   ├── exports.rs       # 5 个导入导出命令
│   └── settings.rs      # 3 个设置命令
└── db/                  # 数据访问层 (纯 SQL)
    ├── rooms.rs         # 机房 CRUD + 级联处理
    ├── racks.rs         # 机柜 CRUD + HashMap 查询 + find_or_create
    ├── devices.rs       # 设备 CRUD + 三重查重 + 日期解析
    ├── device_models.rs # 型号 CRUD + HashMap 查询 + find_or_create
    └── settings.rs      # 键值对设置存储
```

### 前端架构

```
frontend/src/
├── main.tsx             # React 入口 + Ant Design 主题配置
├── App.tsx              # 路由定义 (/racks, /devices)
├── types/index.ts       # 全局 TypeScript 类型
├── tauri-api.ts         # 22 个 Tauri IPC API 封装
├── constants/labels.ts  # 中文标签常量
├── contexts/
│   └── ThemeContext.tsx  # Dark/Light 主题上下文
├── hooks/
│   ├── useApiList.ts    # 通用 CRUD Hook (支持乐观更新)
│   ├── useDevices.ts    # 设备数据 Hook (乐观更新)
│   ├── useRacks.ts      # 机柜数据 Hook (含 updateQuiet)
│   ├── useRooms.ts      # 机房数据 Hook
│   └── useDeviceModels.ts # 型号数据 Hook
├── components/
│   ├── Layout.tsx       # 全局布局 + 设置弹窗 + 主题切换
│   ├── RoomTabs.tsx     # 机房标签页 (拖拽排序)
│   ├── StatusBar.tsx    # 底部状态栏 (缩放/搜索/统计)
│   └── DeviceDetailPanel.tsx # 设备详情侧边栏
├── pages/
│   ├── RackView.tsx     # 机柜可视化核心页面
│   └── DeviceList.tsx   # 设备台账页面
└── styles/
    └── global.css       # 全局样式 (2000+ 行)
```

### 数据库设计

```sql
-- 机房
CREATE TABLE rooms (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    location    TEXT DEFAULT '',
    sort_order  INTEGER DEFAULT 0
);

-- 机柜
CREATE TABLE racks (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    height_u    INTEGER DEFAULT 42,
    row         INTEGER DEFAULT 0,
    col         INTEGER DEFAULT 0,
    view        TEXT DEFAULT 'front',
    sort_order  INTEGER DEFAULT 0,
    room_id     INTEGER REFERENCES rooms(id) ON DELETE SET NULL
);

-- 设备型号
CREATE TABLE device_models (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    manufacturer TEXT DEFAULT '',
    type        TEXT DEFAULT 'server',
    height_u    INTEGER DEFAULT 1,
    power_watt  INTEGER DEFAULT 0
);

-- 设备
CREATE TABLE devices (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    device_model_id INTEGER REFERENCES device_models(id) ON DELETE SET NULL,
    rack_id         INTEGER REFERENCES racks(id) ON DELETE SET NULL,
    start_u         INTEGER,
    end_u           INTEGER,
    ip_addresses    TEXT DEFAULT '',
    serial_no       TEXT DEFAULT '',
    asset_no        TEXT DEFAULT '',
    department      TEXT DEFAULT '',
    owner           TEXT DEFAULT '',
    function        TEXT DEFAULT '',
    purchase_date   TEXT,
    warranty_expire TEXT,
    status          TEXT DEFAULT 'unconfigured',
    power_watt      INTEGER DEFAULT 0
);

-- 应用设置
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 关键设计决策

| 决策 | 理由 |
|------|------|
| **r2d2 连接池替代 Mutex** | 并发读性能提升 4x，PRAGMA 通过 ConnectionCustomizer 持久化 |
| **COALESCE 部分更新** | Update 结构体所有字段 Option，SQL 用 `COALESCE(?n, col)` 只更新非空字段 |
| **软级联删除** | 删除 Room/Rack/Model 时子表外键置 NULL，设备不丢失 |
| **snake_case 序列化** | 前后端字段名统一，避免 174 处前端引用改动 |
| **文件路径传输** | 导出时 Rust 侧弹出原生保存对话框直接写文件，避免二进制 IPC 序列化 |
| **Tauri IPC 替代 HTTP** | invoke() 调用零网络开销，启动无需等待后端服务 |
| **dragDropEnabled=false** | 禁用 Tauri 默认拖放拦截，恢复 HTML5 原生拖拽 |
| **useRef 稳定引用** | 解决 useCallback 依赖不稳定函数引用导致的无限刷新 |

---

## 四、性能表现

| 指标 | 数值 |
|------|------|
| **安装包大小** | 18 MB |
| **启动速度** | < 500ms |
| **运行内存** | 30-50 MB |
| **数据库并发** | 4 连接池 (可调) |
| **导入上限** | 5000 行/次 |
| **日志保留** | 7 天自动清理 |

### 与传统方案对比

| 指标 | Python + pywebview | **Rust + Tauri** | 提升 |
|------|-------------------|-----------------|------|
| 安装包 | 100+ MB | **18 MB** | 🚀 82%↓ |
| 启动速度 | 3-5s | **< 500ms** | 🚀 6-10x |
| 运行内存 | 150-240 MB | **30-50 MB** | 🚀 80%↓ |
| 并发能力 | Mutex 串行 | **r2d2 连接池** | 🚀 4x |

---

## 五、安全与可靠性

| 特性 | 说明 |
|------|------|
| **参数化查询** | 所有 SQL 均使用 `?1` 占位符，零注入风险 |
| **事务保护** | 写操作使用 `BEGIN IMMEDIATE` 事务，失败自动回滚 |
| **外键约束** | SQLite `PRAGMA foreign_keys=ON`，数据一致性保障 |
| **WAL 模式** | `PRAGMA journal_mode=WAL`，读写不阻塞 |
| **busy_timeout** | 5000ms 锁等待，避免并发写冲突 |
| **日志默认关闭** | 隐私友好，用户主动开启才开始记录 |
| **三重查重** | 导入时序列号/资产编号/名称三级去重 |

---

## 六、系统要求

| 项目 | 要求 |
|------|------|
| **操作系统** | Windows 10/11 (x64) |
| **磁盘空间** | 50 MB (含数据库) |
| **内存** | 512 MB+ |
| **显示** | 1024×680 最小分辨率 |

---

## 七、版本历史

### v1.1 (2026-06-10) — Tauri 重构版

- 🏗️ **架构重构**：从 Python FastAPI + pywebview 完整迁移到 Rust + Tauri 2.x
- ⚡ **性能飞跃**：安装包缩小 82%，启动速度提升 10 倍，内存降低 80%
- 🔄 **拖拽上下架**：全新拖拽系统，资源池↔机柜↔机柜自由迁移
- 📥 **智能导入**：三重查重 + 自动创建关联实体 + 结果统计
- ⚙️ **日志系统**：可开关的日志收集，运行时切换，按天轮转
- 🌓 **主题切换**：Dark/Light 全组件适配
- 🔒 **安全加固**：参数化查询 + 事务保护 + 软级联删除

---

## 八、开源依赖

| 库 | 许可证 | 用途 |
|----|--------|------|
| Tauri | MIT/Apache-2.0 | 桌面应用框架 |
| React | MIT | UI 框架 |
| Ant Design | MIT | UI 组件库 |
| rusqlite | MIT | SQLite 绑定 |
| r2d2 | MIT/Apache-2.0 | 连接池 |
| calamine | MIT/Apache-2.0 | Excel 读取 |
| rust_xlsxwriter | BSD-2-Clause | Excel 写入 |
| Askama | MIT/Apache-2.0 | 模板引擎 |
| chrono | MIT/Apache-2.0 | 日期处理 |
| flexi_logger | MIT/Apache-2.0 | 日志系统 |
| Vite | MIT | 构建工具 |

---

*© 2026 RackViz. All rights reserved.*
