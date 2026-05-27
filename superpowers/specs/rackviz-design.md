# RackViz — 机房设备管理系统 设计文档

## 1. 项目概述

参考 RackTables，制作机房设备管理桌面应用，清晰展示机柜使用情况和设备资产。

**用户规模：** 个人/小团队（1-10人），不做多用户权限。

**运行形态：** Web开发模式（Vite + FastAPI）+ Tauri桌面打包。

---

## 2. 技术栈

| 层 | 选择 | 说明 |
|---|------|------|
| 前端 | React 18 + TypeScript + Vite | 不变 |
| 状态管理 | TanStack Query v5 | 缓存、乐观更新、去重请求 |
| UI | Ant Design 5 | 表格/表单/Modal |
| 2D可视化 | React-Konva | 机柜图 + 拓扑图 |
| 3D预览 | React Three Fiber（可选） | 只读机柜预览 |
| 拓扑布局 | dagre | 自动布局有向图 |
| 终端 | xterm.js | SSH会话嵌入 |
| 后端 | Python FastAPI | Sidecar进程 |
| ORM | SQLAlchemy 2 + Alembic | 强制迁移 |
| 数据库 | SQLite（默认）/ PostgreSQL（可选） | 通过 DATABASE_URL 切换 |
| 后台任务 | APScheduler | ICMP定时探测 |
| SSH | asyncssh（后端） | WebSocket代理 |
| 报表 | openpyxl + python-docx + weasyprint | Excel/Word/PDF |
| 桌面壳 | Tauri 2 | 跨平台打包、系统托盘、无边框 |
| 样式 | CSS Variables | 深浅主题token驱动 |

---

## 3. 设计方向

**Aesthetic: Industrial / Utilitarian**

- 默认暗色，浅色为同一token体系反转
- 字体：JetBrains Mono（数据/终端）+ Space Grotesk（标题）
- 配色：暗底 `#0a0b0f` / 浅底 `#f4f5f7`，强调色 `#3dc9b0`
- 设备类型色：蓝=服务器、青=交换机、琥珀=路由、紫=存储、红=PDU、灰=配线架
- 状态色：绿脉冲=在线、红=离线、灰=未配
- 拓扑图：深灰画布 + 类型色节点 + 贝塞尔曲线连线
- 小圆角（2-4px），拒绝"AI审美疲劳"

---

## 4. 数据模型

### 4.1 核心表（10张）

```
DeviceModel（设备模板）
├── id, name, manufacturer, type, height_u, power_watt
├── port_count_front, port_count_rear（快捷字段）

Rack（机柜）
├── id, name, height_u (6-52), row (0-31), col (0-31), view (front/rear)

Device（设备实例）
├── id, name, device_model_id → DeviceModel, rack_id → Rack
├── start_u, end_u, serial_no, function
├── purchase_date, warranty_expire
├── status (online/offline/unconfigured), power_watt

Vlan（VLAN）
├── id, vlan_id (1-4094), name, subnet, color, description

IpAddress（IP地址）
├── id, address（唯一）, device_id → Device（nullable）
├── vlan_id → Vlan（nullable）, label, is_primary

PortTemplate（端口模板，挂在DeviceModel）
├── id, device_model_id → DeviceModel
├── name, port_type (ethernet/fiber/console/sfp+), side (front/rear)
├── speed_mbps, index_no

DevicePort（设备实例端口）
├── id, device_id → Device, port_template_id → PortTemplate（nullable）
├── name, status (up/down/disabled)

Connection（设备连线）
├── id, source_port_id → DevicePort, target_port_id → DevicePort
├── cable_type (cat6/fiber/dac/virtual), label
├── UNIQUE约束每端口一条连线

AuditLog（操作审计，支撑撤销/重做）
├── id, action, entity_type, entity_id
├── payload_before (JSON), payload_after (JSON), created_at
```

### 4.2 关系

```
DeviceModel 1──N PortTemplate
DeviceModel 1──N Device
Rack        1──N Device
Device      1──N DevicePort
Device      1──N IpAddress
DevicePort  ⇄   Connection (source/target)
Vlan        1──N IpAddress
```

### 4.3 约束规则

1. **U位碰撞**：同一rack内[start_u, end_u]不得重叠
2. **end_u计算**：end_u = start_u + device_model.height_u - 1
3. **机柜网格唯一**：(row, col, view)三元组唯一
4. **IP唯一**：address全局唯一；每设备最多一个is_primary
5. **连线规则**：禁止自连；禁止重复连接；允许跨机柜
6. **删除保护**：删设备→级联删端口/IP/连线；删模板→若有设备引用则拒绝

---

## 5. 页面结构

| 页面 | 路由 | 功能 |
|------|------|------|
| 机柜图 | `/racks` | 主工作区，Konva网格，拖拽/多选/撤销，正/背视图 |
| 设备台账 | `/devices` | 表格视图，搜索筛选，批量操作 |
| 设备库 | `/models` | 模板CRUD + 端口模板编辑 |
| 网络拓扑 | `/topology` | Konva有向图，dagre布局，VLAN筛选 |
| IP管理 | `/ips` | IP/VLAN台账，inline编辑 |
| 设置 | `/settings` | 主题、ICMP间隔、网格尺寸、锁屏PIN |

### 5.1 机柜图页面布局

```
┌─────────────────────────────────────────────────────────┐
│ Header: Logo | 导航标签 | ICMP ● | 导出 ▾ | 锁屏        │
├─────────────────────────────────────────────────────────┤
│ Toolbar: [拖拽|编辑|删除|连线] | ↩↪ | 缩放 | 正/背      │
│                          [+机柜] | [搜索...Ctrl+F]      │
├───────────────────────────────┬─────────────────────────┤
│  机柜网格 (Konva Stage+视口)  │ 侧栏:                   │
│  ┌──┬──┬──┐                  │  设备库（拖放源）        │
│  │01│02│03│ ← Ctrl+点击多选  │  设备详情                │
│  └──┴──┴──┘                  │  端口列表                │
├───────────────────────────────┴─────────────────────────┤
│ Statusbar: 缩放% | 机柜 | 设备 | 功率 | 在线 | 模式     │
└─────────────────────────────────────────────────────────┘
```

### 5.2 网络拓扑页面

```
┌─────────────────────────────────────────────────────────┐
│ Toolbar: VLAN筛选 | 布局选择 | 缩放 | 导出PNG           │
├─────────────────────────────────────────────────────────┤
│     ┌───┐    贝塞尔     ┌───┐                            │
│     │ S │───────────────│ R │   Konva有向图             │
│     └───┘               └───┘   双击跳转机柜图           │
├─────────────────────────────────────────────────────────┤
│ Statusbar: 节点N | 连线E | VLAN                         │
└─────────────────────────────────────────────────────────┘
```

---

## 6. 核心交互

### 6.1 机柜图

| 交互 | 操作 | 说明 |
|------|------|------|
| 拖拽布放 | 设备库→机柜U位 | 创建Device |
| 机柜内移动 | 拖拽设备块 | 改变U位 |
| 机柜间迁移 | 横拖设备块 | 改变rack_id+U位 |
| 双击编辑 | 双击设备/机柜头 | 弹出编辑Modal |
| 删除 | 切换到删除模式，双击设备 | 确认后删除 |
| 撤销/重做 | Ctrl+Z / Ctrl+Y | 基于AuditLog |
| 多选 | Ctrl+点击 | 批量移动/删除 |
| 缩放 | Ctrl+滚轮 | 全局缩放 |
| 搜索 | Ctrl+F | 定位+高亮 |
| 视图切换 | 工具栏 | 正/背视图过滤 |
| 连线模式 | 工具栏切换 | 选端口→画线→保存 |
| 锁屏 | Ctrl+L | PIN解锁 |

### 6.2 拓扑图

| 交互 | 操作 |
|------|------|
| 拖放节点 | 微调布局 |
| 点击连线 | 高亮端口 |
| 双击节点 | 跳转机柜图 |
| VLAN筛选 | 过滤显示 |

### 6.3 桌面

| 交互 | 说明 |
|------|------|
| 托盘 | 右键菜单，ICMP摘要 |
| 最小化 | 关闭→隐藏到托盘 |
| 沉浸 | 隐藏标题栏，自定义控件 |

---

## 7. 开发阶段

### Phase 0 — 后端基础
- 项目脚手架、配置、数据库连接
- 全部10张表模型 + Alembic迁移
- 共享常量（前后端type colors等）
- ICMP探测服务（并发+APScheduler）

### Phase 1 — 后端API
- 全部CRUD API（Rack, Device, DeviceModel, Vlan, IpAddress, PortTemplate, DevicePort, Connection）
- U位碰撞检测
- AuditLog + 撤销/重做
- Excel导入/导出
- 拓扑数据接口

### Phase 2 — 前端基础
- Vite + React + TypeScript 脚手架
- TanStack Query + API client
- Layout shell（Header, Toolbar, StatusBar）
- 主题系统（暗/浅）
- 路由框架

### Phase 3 — 机柜编辑器
- Konva机柜网格渲染
- 设备拖拽（机柜内+机柜间）
- 设备库面板（拖拽源）
- 设备详情面板
- 编辑/删除Modal

### Phase 4 — 编辑器增强
- 撤销/重做UI
- 多选+批量操作
- 搜索定位+高亮
- 视口裁剪优化
- 机柜管理（新建/删除/复制）
- 正/背视图过滤

### Phase 5 — 其余页面
- 设备台账页（表格+搜索+批量删除）
- 设备库页（表格+端口模板子表）
- IP管理页（IP+VLAN双标签）
- 设置页（主题/网格/ICMP间隔/PIN）

### Phase 6 — 网络拓扑
- 拓扑API（节点+边构建）
- 拓扑页面（Konva + dagre）
- VLAN筛选 + 双击跳转

### Phase 7 — 端口和连线
- 设备库端口模板编辑
- 设备端口同步
- 连线模式（画线+保存）
- 拓扑同步连线数据

### Phase 8 — 报表和SSH
- 多格式报表（Excel/Word/PDF/HTML）
- WebSocket SSH终端（xterm.js）
- 锁屏功能

### Phase 9 — Tauri桌面
- Tauri 2项目 + Sidecar打包
- 系统托盘
- 沉浸标题栏
- 构建脚本

### Phase 10 — 测试和发布
- pytest + Vitest + Playwright E2E
- 种子数据
- CI pipeline
- 验收测试

---

## 8. API概览

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST/PUT/DELETE | `/api/racks` | 机柜CRUD |
| GET/POST/PUT/DELETE | `/api/devices` | 设备CRUD |
| GET/POST/PUT/DELETE | `/api/device-models` | 模板CRUD |
| GET/POST/PUT/DELETE | `/api/vlans` | VLAN CRUD |
| GET/POST/PUT/DELETE | `/api/ip-addresses` | IP CRUD |
| GET/POST/DELETE | `/api/device-models/{id}/ports` | 端口模板 |
| GET/POST | `/api/devices/{id}/ports` | 设备端口 |
| GET/POST/DELETE | `/api/connections` | 连线 |
| GET | `/api/topology` | 拓扑数据 |
| POST | `/api/audit/undo`, `/api/audit/redo` | 撤销重做 |
| GET | `/api/export/racks.xlsx` | Excel导出 |
| POST | `/api/export/import` | Excel导入 |
| GET | `/api/export/report.{docx,pdf,html}` | 报表 |
| POST | `/api/devices/refresh-status` | ICMP刷新 |
| WS | `/ws/ssh/{device_id}` | SSH终端 |

---

## 9. 设计决策

| 决策 | 理由 |
|------|------|
| SQLite默认 | 零配置，小团队够用；PG改连接串切换 |
| 不用Tailwind | 工业美学需要精细控制，CSS Variables可控 |
| Canvas（Konva）而非DOM | 大数据量机柜图DOM性能差 |
| 设备库与设备分离 | 模板重用 |
| IP独立表 | 支撑VLAN/拓扑/主IP探测 |
| AuditLog持久化 | 撤销重启后仍可追溯 |
| dagre自动布局 | 拓扑节点<100时性能足够 |
| Tauri Sidecar | 复用Python业务逻辑，降低迁移成本 |
| SSH凭据本地keychain | 安全，不上传 |
| 不做RBAC | 个人/小团队不需要 |

---

## 10. 验收标准

1. 52U/32×32网格机柜图正常渲染，拖拽布放正常
2. U位碰撞被API拒绝（409）
3. Ctrl+Z撤销设备移动
4. 正/背视图切换正确过滤
5. ICMP探测状态标签自动更新
6. 拓扑图展示连线，VLAN筛选有效
7. Excel导出机柜图1:1还原
8. 从设备详情可SSH连接（测试环境）
9. Word/PDF/HTML报表包含机柜图+设备台账
10. 锁屏需PIN恢复
11. Tauri安装包启动后无需手动启后端
12. 系统托盘+最小化到托盘
13. 全部pytest+Vitest+Playwright通过
