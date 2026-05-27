# RackViz — 机房设备管理系统 v2.0 设计文档

## 1. 版本定位

### 1.1 前提

v2.0 建立在 **v1.0 已完成** 的基础上。v1.0 交付：机柜 Canvas 可视化、设备/机柜/模板 CRUD、ICMP 状态探测、Excel 导入导出。

v1.0 评审中识别的技术债，在 v2.0 **Phase 0** 中优先偿还（U 位碰撞检测、Alembic 迁移、React Query 状态层、ICMP 并发）。

### 1.2 v2.0 目标

从「单机 Web 机柜图工具」升级为 **桌面原生 + 网络拓扑 + 端口连线** 的机房资产管理平台。

| 维度 | v1.0 | v2.0 |
|------|------|------|
| 运行形态 | 浏览器 + 本地 FastAPI | Tauri 桌面应用（内嵌 WebView + Sidecar 后端） |
| 机柜编辑 | 基础拖拽 | 撤销/重做、机柜管理、多选、正/背视图、52U/32×32 |
| IP 管理 | 分号分隔字符串 | 独立 IP 地址表 + VLAN |
| 可视化 | 2D 机柜网格 | 2D 机柜 + **网络拓扑图** |
| 设备模型 | 模板 U 数/功率 | 模板 + **端口库** + 设备间连线 |
| 导出 | Excel | Excel + Word/PDF/HTML 报表 |
| 主题 | 暗色固定 | 暗色/浅色切换 |
| 远程运维 | 无 | SSH 终端（xterm.js） |
| 3D | 无 | 可选预览（Three.js 简化 rack view，非必须） |

### 1.3 用户规模

个人 / 小团队（1–10 人）。v2.0 **仍不做多用户权限**；v3.0 再考虑 RBAC。

### 1.4 版本策略

```
v1.0 ──► v2.0 Phase 0（技术债）
              │
              ├── Phase 1：编辑器增强（原 v1.1 核心）
              ├── Phase 2：Tauri 桌面化
              ├── Phase 3：网络管理 + 拓扑图
              ├── Phase 4：端口库 + 设备连线
              ├── Phase 5：报表导出 + SSH 终端
              └── Phase 6（可选）：3D 机柜预览
```

---

## 2. 技术栈

### 2.1 延续（v1.0）

| 层 | 选择 | 说明 |
|---|------|------|
| 前端 | React 18 + TypeScript + Vite | 不变 |
| UI | Ant Design 5 | 表格/表单/Modal |
| 2D 可视化 | React-Konva | 机柜图 + 拓扑图 |
| 后端 | Python FastAPI | Sidecar 进程 |
| ORM | SQLAlchemy 2 + Alembic | **强制迁移，禁止 create_all** |
| 数据库 | SQLite（默认）/ PostgreSQL（可选） | 通过 `DATABASE_URL` 切换 |
| 样式 | CSS Variables | 主题 token 驱动深浅模式 |

### 2.2 v2.0 新增

| 层 | 选择 | 说明 |
|---|------|------|
| 桌面壳 | **Tauri 2** | 跨平台打包，系统托盘，无边框窗口 |
| 状态管理 | **TanStack Query v5** | 替代手写 hooks，缓存/乐观更新 |
| 拓扑布局 | **dagre**（或 elkjs） | 自动布局有向图 |
| 终端 | **xterm.js** + `@xterm/addon-fit` | SSH 会话嵌入 |
| SSH | **asyncssh**（后端） | WebSocket 代理到 SSH |
| 3D（可选） | **React Three Fiber** | 只读机柜预览 |
| 报表 | **python-docx**、**weasyprint**（或 reportlab） | Word/PDF |
| HTML 报表 | Jinja2 模板 | 服务端渲染静态 HTML |
| 任务队列 | **APScheduler**（内置） | ICMP 定时探测，不阻塞 API |
| 测试 | pytest + Vitest + Playwright | 后端/前端/E2E |

---

## 3. 设计方向

### 3.1 美学（延续 Industrial / Utilitarian）

- 默认暗色；浅色模式为同一 token 体系的反转，非简单反色
- 字体：JetBrains Mono（数据/终端）+ Space Grotesk（标题/导航）
- 配色：暗底 `#0a0b0f` / 浅底 `#f4f5f7`，强调 `#3dc9b0`
- 拓扑图：深灰画布 + 类型色节点 + 连线动画（流量方向可选 v2.1）

### 3.2 新增视觉元素

| 元素 | 说明 |
|------|------|
| 连线 | 贝塞尔曲线，hover 显示端口标签 |
| VLAN 标签 | 小胶囊 badge，颜色按 VLAN ID 哈希 |
| 多选框 | 半透明青绿描边 |
| 撤销指示 | StatusBar 短暂 toast「已撤销：移动设备 X」 |
| 托盘图标 | 简化机柜轮廓 + 在线设备数 badge |

---

## 4. 数据模型

### 4.1 v1.0 表（保留，字段变更见迁移说明）

- `device_models` — 设备模板
- `racks` — 机柜
- `devices` — 设备实例

**v2.0 变更：**

| 表/字段 | 变更 |
|---------|------|
| `devices.ip_addresses` | **废弃**（迁移到 `ip_addresses` 表，保留列只读兼容一版本） |
| `racks.height_u` | 上限扩展到 **52** |
| `racks.row`, `racks.col` | 上限扩展到 **31**（32×32 网格） |
| `device_models` | 新增 `port_count_front`, `port_count_rear`（可选快捷字段） |

### 4.2 新增表

```
IpAddress（IP 地址）
├── id
├── address          # 192.168.1.100，唯一
├── device_id → Device（nullable，未分配设备）
├── vlan_id → Vlan（nullable）
├── label            # 用途说明，如「管理口」
├── is_primary       # 是否主 IP（ICMP 探测用）
├── created_at

Vlan（VLAN）
├── id
├── vlan_id          # 802.1Q ID，1-4094
├── name             # 「管理网」
├── subnet           # 192.168.1.0/24（展示用，不做 IPAM 计算）
├── color            # 拓扑/UI 标签色
├── description

PortTemplate（端口模板 — 挂在 DeviceModel）
├── id
├── device_model_id → DeviceModel
├── name             # GE1/0/1, eth0
├── port_type        # ethernet/fiber/console/power/sfp+
├── side             # front/rear/both
├── speed_mbps       # 1000, 10000, ...
├── index_no         # 排序

DevicePort（设备实例端口）
├── id
├── device_id → Device
├── port_template_id → PortTemplate（nullable）
├── name             # 可覆盖模板名
├── status           # up/down/disabled

Connection（设备连线）
├── id
├── source_port_id → DevicePort
├── target_port_id → DevicePort
├── cable_type       # cat6/fiber/dac/virtual
├── label            # 可选描述
├── created_at
├── UNIQUE(source_port_id), UNIQUE(target_port_id)  # 每端口最多一条连线

AuditLog（操作审计 — 支撑撤销/重做）
├── id
├── action           # create/update/delete/move/connect
├── entity_type      # device/rack/connection/...
├── entity_id
├── payload_before   # JSON
├── payload_after    # JSON
├── created_at
```

### 4.3 关系图

```
DeviceModel 1──N PortTemplate
DeviceModel 1──N Device
Rack        1──N Device
Device      1──N DevicePort
Device      1──N IpAddress
DevicePort  1──1 Connection (as source or target)
Vlan        1──N IpAddress
```

### 4.4 约束与业务规则

1. **U 位碰撞**：同一 `rack_id` 内 `[start_u, end_u]` 区间不得重叠（v2 Phase 0 起强制）
2. **end_u 计算**：`end_u = start_u + device_model.height_u - 1`，由服务端写入
3. **机柜网格**：`(row, col, view)` 三元组唯一
4. **IP 唯一**：`address` 全局唯一；每设备最多一个 `is_primary=true`
5. **连线**：禁止自连；禁止重复连接同一端口；跨机柜连线允许
6. **删除级联**：删设备 → 删其端口、IP、相关连线；删模板 → 若仍有设备引用则拒绝

---

## 5. 页面结构

### 5.1 页面清单

| 页面 | 路由 | 功能 |
|------|------|------|
| 机柜图 | `/racks` | 2D 网格，拖拽/多选/撤销，正/背视图 |
| 设备台账 | `/devices` | 表格，筛选，批量操作 |
| 设备库 | `/models` | 模板 CRUD + **端口模板编辑** |
| 网络拓扑 | `/topology` | **新增** — VLAN 过滤，自动布局，连线编辑 |
| IP 管理 | `/ips` | **新增** — IP/VLAN 台账 |
| 设置 | `/settings` | 主题、ICMP 间隔、网格尺寸、锁屏 PIN |

### 5.2 机柜图页面（增强）

```
┌──────────────────────────────────────────────────────────────┐
│ Header: Logo | 导航 | ICMP ● | 导出 ▾ | 锁屏 | ─ □ ✕ (Tauri) │
├──────────────────────────────────────────────────────────────┤
│ Toolbar: [拖拽|编辑|删除|连线] | ↩↪ | 缩放 | 正/背 | [+机柜]   │
│                                              [搜索设备...]    │
├────────────────────────────────────────┬─────────────────────┤
│  机柜网格 (Konva Stage + 视口裁剪)      │ 设备库（拖放源）     │
│  ┌──┬──┬──┬──┐                         │ ─────────────────   │
│  │01│02│03│04│  ← Ctrl+点击多选         │ 设备详情 / 端口列表  │
│  └──┴──┴──┴──┘                         │ 选中设备时           │
├────────────────────────────────────────┴─────────────────────┤
│ Statusbar: 缩放 | 机柜 | 设备 | 功率 | 在线 | 模式 | Ctrl+Z   │
└──────────────────────────────────────────────────────────────┘
```

### 5.3 网络拓扑页面

```
┌──────────────────────────────────────────────────────────────┐
│ Header（同上）                                                │
├──────────────────────────────────────────────────────────────┤
│ Toolbar: VLAN 筛选 | 布局[自动|树形|力导向] | 缩放 | 导出 PNG  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│     ┌─────────┐         ┌─────────┐                          │
│     │ Switch1 │─────────│ Server1 │   Konva 有向图            │
│     └────┬────┘         └─────────┘                          │
│          │                                                   │
│     ┌────┴────┐                                                │
│     │ Router1 │                                                │
│     └─────────┘                                                │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ Statusbar: 节点数 | 连线数 | 当前 VLAN                       │
└──────────────────────────────────────────────────────────────┘
```

### 5.4 SSH 终端（抽屉/面板）

- 从设备详情点击「打开终端」→ 右侧 Drawer 弹出 xterm
- 使用设备 `is_primary` IP + 凭据（凭据存本地 keychain，via Tauri plugin，**不上传服务器**）

---

## 6. 核心交互

### 6.1 机柜图（v1.0 增强）

| 交互 | 操作 | v2.0 说明 |
|------|------|-----------|
| 撤销/重做 | Ctrl+Z / Ctrl+Y | 基于 AuditLog，最近 50 步 |
| 多选 | Ctrl+点击 | 批量移动/删除 |
| 机柜管理 | 工具栏「+机柜」 | 选行/列/U 高，支持复制机柜 |
| 正/背视图 | 工具栏切换 | 按 `rack.view` 过滤 |
| 网格扩展 | 设置页 | 24×24 或 32×32；6–52U |
| 连线模式 | 工具栏 | 选源端口→目标端口，Canvas 上显示跳线 |
| 锁屏 | Ctrl+L | 全屏遮罩 + PIN |

### 6.2 拓扑图

| 交互 | 操作 |
|------|------|
| 拖放节点 | 手动微调自动布局结果 |
| 点击连线 | 高亮两端端口，侧栏显示详情 |
| 双击节点 | 跳转机柜图并选中设备 |
| VLAN 筛选 | 只显示该 VLAN 下的 IP 关联设备 |

### 6.3 桌面特有

| 交互 | 说明 |
|------|------|
| 托盘 | 右键：显示/隐藏、ICMP 状态摘要、退出 |
| 最小化到托盘 | 关闭窗口默认隐藏而非退出 |
| 沉浸模式 | 隐藏 Tauri 标题栏，自定义窗口控件 |
| 开机自启 | 可选，设置页开关 |

---

## 7. v2.0 范围

### 7.1 必须实现（P0–P5）

**Phase 0 — 基础加固**
- [ ] Alembic 初始迁移 + 后续版本迁移
- [ ] U 位碰撞检测服务
- [ ] TanStack Query 替换手写 hooks
- [ ] ICMP 并发探测 + APScheduler 后台任务
- [ ] 共享常量（前后端 type colors、U 尺寸）

**Phase 1 — 编辑器增强**
- [ ] 撤销/重做（AuditLog）
- [ ] 机柜 CRUD UI（新建/删除/复制）
- [ ] 正/背视图过滤
- [ ] 52U / 32×32 网格
- [ ] Ctrl+多选 + 批量操作
- [ ] 深浅主题切换
- [ ] 功率汇总（模板 fallback）
- [ ] 搜索定位（高亮 + 平移 Canvas）

**Phase 2 — Tauri 桌面**
- [ ] Tauri 2 项目 + Sidecar FastAPI
- [ ] 系统托盘 + 最小化到托盘
- [ ] 自定义标题栏 / 沉浸模式
- [ ] 生产构建脚本（Win/macOS/Linux）

**Phase 3 — 网络管理**
- [ ] IpAddress / Vlan CRUD API + 页面
- [ ] IP 从 `devices.ip_addresses` 迁移
- [ ] 拓扑图页面（Konva + dagre）
- [ ] VLAN 着色与筛选

**Phase 4 — 端口与连线**
- [ ] PortTemplate / DevicePort / Connection 模型与 API
- [ ] 设备库端口模板编辑 UI
- [ ] 机柜图连线模式
- [ ] 拓扑图与 Connection 数据同步

**Phase 5 — 导出与终端**
- [ ] Word / PDF / HTML 报表
- [ ] Excel 导出按 row/col 网格 1:1 还原
- [ ] SSH 终端（WebSocket + xterm.js）
- [ ] 锁屏（Ctrl+L）

### 7.2 可选（Phase 6 — 不阻塞 v2.0 发布）

- [ ] Three.js 3D 机柜预览（只读）
- [ ] 拓扑流量动画
- [ ] PostgreSQL 一键切换文档 + CI 矩阵测试

### 7.3 明确不做（v3.0+）

- 多用户 RBAC / OAuth
- 完整 IPAM（子网分配、DHCP 集成）
- NetBox / Zabbix 双向同步
- 移动端适配

---

## 8. API 概览（v2.0 新增）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/api/vlans` | VLAN CRUD |
| GET/POST | `/api/ip-addresses` | IP CRUD |
| GET/POST | `/api/device-models/{id}/ports` | 端口模板 |
| GET/POST | `/api/devices/{id}/ports` | 设备端口 |
| GET/POST/DELETE | `/api/connections` | 连线 |
| GET | `/api/topology` | 拓扑图数据（节点+边） |
| POST | `/api/audit/undo` | 撤销 |
| POST | `/api/audit/redo` | 重做 |
| GET | `/api/export/report.{docx,pdf,html}` | 报表 |
| WS | `/ws/ssh/{device_id}` | SSH 终端 |
| GET | `/api/jobs/icmp/status` | 后台探测状态 |

**版本策略**：v2.0 API 保持 `/api` 前缀，破坏性变更通过 Pydantic 字段默认值和迁移脚本兼容 v1.0 客户端（仅 Web 降级模式）。

---

## 9. 设计决策记录

| 决策 | 理由 |
|------|------|
| Tauri Sidecar 而非 Rust 重写后端 | 复用 v1.0 Python 业务逻辑，降低迁移成本 |
| IP 独立表 | 支撑 VLAN、拓扑、主 IP 探测；告别分号字符串 |
| AuditLog 而非 Command 模式内存栈 | 持久化撤销，重启后仍可追溯（最近 N 条可重做） |
| dagre 自动布局 | 拓扑节点数 <100 时性能足够，实现快 |
| SSH 凭据存 Tauri keychain | 安全；小团队无集中密钥管理 |
| 3D 为可选 Phase | 投入产出比低，2D 已满足核心场景 |
| 仍不做 RBAC | v2.0 聚焦功能广度，权限留 v3.0 |

---

## 10. 迁移与兼容

### 10.1 数据迁移（Alembic `v1_to_v2`）

1. 创建新表：`vlans`, `ip_addresses`, `port_templates`, `device_ports`, `connections`, `audit_logs`
2. 解析 `devices.ip_addresses`（`;` 分隔）→ 插入 `ip_addresses`，首条 `is_primary=true`
3. 扩展 `racks.height_u` CHECK 6–52；`row/col` CHECK 0–31
4. 标记 `devices.ip_addresses` 为 deprecated（列保留，API 双写一版本后移除）

### 10.2 部署形态

| 形态 | 说明 |
|------|------|
| Web 开发模式 | 同 v1.0，`vite dev` + `uvicorn`，无 Tauri |
| Tauri 开发模式 | `tauri dev`，Sidecar 自动启停 |
| 生产安装包 | `.msi` / `.dmg` / `.AppImage`，内嵌 Python runtime 或 PyInstaller sidecar |

---

## 11. 验收标准

1. Tauri 安装包启动后，无需手动启后端即可打开机柜图
2. 同一机柜 U 位重叠放置被 API 拒绝（409）
3. Ctrl+Z 可撤销最近一次设备移动
4. 正/背视图切换后 Canvas 只显示对应 `rack.view` 机柜
5. IP 管理页 CRUD 后，ICMP 只 ping `is_primary` IP
6. 拓扑图正确展示 Connection 连线，VLAN 筛选有效
7. 从设备详情可打开 SSH 终端并成功连接（测试环境）
8. Word/PDF/HTML 报表包含机柜图快照与设备台账摘要
9. 锁屏后需 PIN 才能恢复操作
10. 全部 pytest + Vitest 通过；Playwright 覆盖拖放/撤销/导出 happy path
