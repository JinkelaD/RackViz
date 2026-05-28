import requests

racks = requests.get("http://127.0.0.1:8000/api/racks/").json()
rooms = requests.get("http://127.0.0.1:8000/api/rooms/").json()
room_ids = {r["id"] for r in rooms}

for rack in racks:
    rid = rack.get("room_id")
    if rid is not None and rid not in room_ids:
        print(f"Fixing orphaned: {rack['name']} (room_id={rid})")
        requests.put(f"http://127.0.0.1:8000/api/racks/{rack['id']}", json={"room_id": None})

print("Cleanup done")
