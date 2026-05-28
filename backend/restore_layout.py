import requests

BASE = "http://127.0.0.1:8000/api"

racks = [
    {"name": "机柜-A01", "height_u": 42, "view": "front", "sort_order": 0},
    {"name": "机柜-A02", "height_u": 42, "view": "front", "sort_order": 1},
    {"name": "机柜-A03", "height_u": 42, "view": "front", "sort_order": 2},
]

print("=== 1. 创建机柜 ===")
rack_ids = []
for r in racks:
    resp = requests.post(f"{BASE}/racks/", json=r)
    if resp.status_code == 201:
        rid = resp.json()["id"]
        rack_ids.append(rid)
        print(f"  ✓ {r['name']} (id={rid})")
    else:
        print(f"  ✗ {r['name']}: {resp.status_code} {resp.text[:100]}")

devices = requests.get(f"{BASE}/devices/").json()

device_plan = [
    {"name": "核心交换机-A", "rack": 0, "u": 42},
    {"name": "核心交换机-B", "rack": 0, "u": 41},
    {"name": "汇聚交换机-A", "rack": 0, "u": 40},
    {"name": "接入交换机-A1", "rack": 1, "u": 42},
    {"name": "接入交换机-A2", "rack": 1, "u": 41},
    {"name": "数据库服务器-01", "rack": 1, "u": 39},
    {"name": "数据库服务器-02", "rack": 1, "u": 37},
    {"name": "虚拟化服务器-01", "rack": 1, "u": 35},
    {"name": "虚拟化服务器-02", "rack": 1, "u": 33},
    {"name": "虚拟化服务器-03", "rack": 1, "u": 31},
    {"name": "备份服务器-01", "rack": 2, "u": 42},
    {"name": "出口路由器-01", "rack": 2, "u": 40},
    {"name": "出口路由器-02", "rack": 2, "u": 38},
    {"name": "SAN存储-01", "rack": 2, "u": 35},
    {"name": "SAN存储-02", "rack": 2, "u": 32},
    {"name": "PDU-A1", "rack": 0, "u": 1},
    {"name": "PDU-A2", "rack": 1, "u": 1},
    {"name": "配线架-A1", "rack": 2, "u": 1},
]

print(f"\n=== 2. 分配设备到机柜 ===")
for plan in device_plan:
    dev = next((d for d in devices if d["name"] == plan["name"]), None)
    if not dev:
        print(f"  ✗ 未找到设备: {plan['name']}")
        continue
    model_id = dev["device_model_id"]
    model = requests.get(f"{BASE}/device-models/{model_id}").json() if model_id else None
    height = model["height_u"] if model else 2
    rack_id = rack_ids[plan["rack"]] if plan["rack"] < len(rack_ids) else None
    if rack_id is None:
        print(f"  ✗ 机柜索引越界: {plan['rack']}")
        continue
    start_u = plan["u"] - height + 1
    resp = requests.put(f"{BASE}/devices/{dev['id']}", json={
        "rack_id": rack_id,
        "start_u": start_u,
        "end_u": plan["u"],
    })
    if resp.status_code == 200:
        print(f"  ✓ {plan['name']} → 机柜{plan['rack']+1} U{start_u}-{plan['u']}")
    else:
        print(f"  ✗ {plan['name']}: {resp.status_code} {resp.text[:100]}")

print(f"\n=== 3. 完成 ===")
racks_resp = requests.get(f"{BASE}/racks/").json()
devices_resp = requests.get(f"{BASE}/devices/").json()
assigned = sum(1 for d in devices_resp if d["rack_id"] is not None)
print(f"  机柜: {len(racks_resp)} 个, 已分配设备: {assigned}/{len(devices_resp)} 台")
