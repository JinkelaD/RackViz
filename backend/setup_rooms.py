import requests

BASE = "http://127.0.0.1:8000/api"

rooms_data = [
    {"name": "A栋-主机房", "location": "A栋1楼", "sort_order": 0},
    {"name": "B栋-容灾机房", "location": "B栋2楼", "sort_order": 1},
]

print("=== 1. 创建机房 ===")
room_ids = []
for r in rooms_data:
    resp = requests.post(f"{BASE}/rooms/", json=r)
    if resp.status_code == 201:
        rid = resp.json()["id"]
        room_ids.append(rid)
        print(f"  ✓ {r['name']} (id={rid})")
    else:
        print(f"  ✗ {r['name']}: {resp.status_code}")

racks = requests.get(f"{BASE}/racks/").json()
print(f"\n=== 2. 分配机柜到机房 (共 {len(racks)} 个机柜) ===")

assignments = {
    "机柜-A01": room_ids[0],
    "机柜-A02": room_ids[0],
    "机柜-A03": room_ids[1],
}

for rack in racks:
    room_id = assignments.get(rack["name"])
    if room_id:
        resp = requests.put(f"{BASE}/racks/{rack['id']}", json={"room_id": room_id})
        print(f"  ✓ {rack['name']} → room_id={room_id}")
    else:
        print(f"  - {rack['name']} (未分配)")

print(f"\n=== 3. 完成 ===")
rooms = requests.get(f"{BASE}/rooms/").json()
for room in rooms:
    room_racks = [r for r in racks if r.get("room_id") == room["id"]]
    print(f"  {room['name']}: {len(room_racks)} 个机柜")
