import requests
import json
from datetime import date, timedelta

BASE = "http://127.0.0.1:8000/api"

models_to_create = [
    {"name": "Dell R750xs", "manufacturer": "Dell", "type": "server", "height_u": 2},
    {"name": "HPE DL380 Gen11", "manufacturer": "HPE", "type": "server", "height_u": 2},
    {"name": "Inspur NF5280M7", "manufacturer": "浪潮", "type": "server", "height_u": 2},
    {"name": "Huawei 2288H V7", "manufacturer": "华为", "type": "server", "height_u": 2},
    {"name": "Cisco Nexus 93180YC", "manufacturer": "Cisco", "type": "switch", "height_u": 1},
    {"name": "Huawei CE6881", "manufacturer": "华为", "type": "switch", "height_u": 1},
    {"name": "H3C S6850", "manufacturer": "H3C", "type": "switch", "height_u": 1},
    {"name": "Huawei AR6300", "manufacturer": "华为", "type": "router", "height_u": 2},
    {"name": "Cisco ISR 4461", "manufacturer": "Cisco", "type": "router", "height_u": 2},
    {"name": "Huawei OceanStor 5310", "manufacturer": "华为", "type": "storage", "height_u": 3},
    {"name": "Dell PowerStore 500T", "manufacturer": "Dell", "type": "storage", "height_u": 2},
    {"name": "APC AP8886", "manufacturer": "APC", "type": "pdu", "height_u": 1},
    {"name": "Vertiv Geist UPDU", "manufacturer": "Vertiv", "type": "pdu", "height_u": 1},
    {"name": "AMP 48-port Patch Panel", "manufacturer": "AMP", "type": "patch", "height_u": 1},
    {"name": "Panduit 24-port Patch Panel", "manufacturer": "Panduit", "type": "patch", "height_u": 1},
]

devices_to_create = [
    {"name": "核心交换机-A", "model": "Cisco Nexus 93180YC", "serial": "SN-CORE-A001", "asset": "ZC-2024-0001", "dept": "网络运维部", "owner": "张伟"},
    {"name": "核心交换机-B", "model": "Cisco Nexus 93180YC", "serial": "SN-CORE-B002", "asset": "ZC-2024-0002", "dept": "网络运维部", "owner": "张伟"},
    {"name": "汇聚交换机-A", "model": "Huawei CE6881", "serial": "SN-AGG-A001", "asset": "ZC-2024-0003", "dept": "网络运维部", "owner": "张伟"},
    {"name": "接入交换机-A1", "model": "H3C S6850", "serial": "SN-ACC-A1001", "asset": "ZC-2024-0004", "dept": "网络运维部", "owner": "赵强"},
    {"name": "接入交换机-A2", "model": "H3C S6850", "serial": "SN-ACC-A1002", "asset": "ZC-2024-0005", "dept": "网络运维部", "owner": "赵强"},
    {"name": "虚拟化服务器-01", "model": "Dell R750xs", "serial": "SN-VM-0001", "asset": "ZC-2024-0006", "dept": "IT基础设施部", "owner": "李娜"},
    {"name": "虚拟化服务器-02", "model": "Dell R750xs", "serial": "SN-VM-0002", "asset": "ZC-2024-0007", "dept": "IT基础设施部", "owner": "李娜"},
    {"name": "虚拟化服务器-03", "model": "Dell R750xs", "serial": "SN-VM-0003", "asset": "ZC-2024-0008", "dept": "IT基础设施部", "owner": "李娜"},
    {"name": "虚拟化服务器-04", "model": "HPE DL380 Gen11", "serial": "SN-VM-0004", "asset": "ZC-2024-0009", "dept": "IT基础设施部", "owner": "李娜"},
    {"name": "数据库服务器-01", "model": "Inspur NF5280M7", "serial": "SN-DB-0001", "asset": "ZC-2024-0010", "dept": "IT基础设施部", "owner": "刘洋"},
    {"name": "数据库服务器-02", "model": "Inspur NF5280M7", "serial": "SN-DB-0002", "asset": "ZC-2024-0011", "dept": "IT基础设施部", "owner": "刘洋"},
    {"name": "备份服务器-01", "model": "Huawei 2288H V7", "serial": "SN-BK-0001", "asset": "ZC-2024-0012", "dept": "存储管理部", "owner": "王磊"},
    {"name": "出口路由器-01", "model": "Huawei AR6300", "serial": "SN-RT-0001", "asset": "ZC-2024-0013", "dept": "网络运维部", "owner": "张伟"},
    {"name": "出口路由器-02", "model": "Cisco ISR 4461", "serial": "SN-RT-0002", "asset": "ZC-2024-0014", "dept": "网络运维部", "owner": "张伟"},
    {"name": "SAN存储-01", "model": "Huawei OceanStor 5310", "serial": "SN-SAN-0001", "asset": "ZC-2024-0015", "dept": "存储管理部", "owner": "王磊"},
    {"name": "SAN存储-02", "model": "Dell PowerStore 500T", "serial": "SN-SAN-0002", "asset": "ZC-2024-0016", "dept": "存储管理部", "owner": "王磊"},
    {"name": "PDU-A1", "model": "APC AP8886", "serial": "SN-PDU-A01", "asset": "ZC-2024-0017", "dept": "机房运维部", "owner": "陈静"},
    {"name": "PDU-A2", "model": "Vertiv Geist UPDU", "serial": "SN-PDU-A02", "asset": "ZC-2024-0018", "dept": "机房运维部", "owner": "陈静"},
    {"name": "配线架-A1", "model": "AMP 48-port Patch Panel", "serial": "SN-PATCH-A01", "asset": "ZC-2024-0019", "dept": "综合布线组", "owner": "陈静"},
    {"name": "配线架-A2", "model": "Panduit 24-port Patch Panel", "serial": "SN-PATCH-A02", "asset": "ZC-2024-0020", "dept": "综合布线组", "owner": "陈静"},
]

def main():
    print("=== 1. 创建设备型号 ===")
    model_map = {}
    for m in models_to_create:
        resp = requests.post(f"{BASE}/device-models/", json=m)
        if resp.status_code == 201:
            mid = resp.json()["id"]
            model_map[m["name"]] = mid
            print(f"  ✓ {m['name']} ({m['type']}, {m['height_u']}U)")
        else:
            print(f"  ✗ {m['name']}: {resp.status_code} {resp.text[:100]}")

    print(f"\n=== 2. 创建设备 (共 {len(devices_to_create)} 台) ===")
    ip_parts = [10, 21, 173, 1]
    def next_ip():
        ip_parts[3] += 1
        if ip_parts[3] > 254:
            ip_parts[3] = 1
            ip_parts[2] += 1
        return f"{ip_parts[0]}.{ip_parts[1]}.{ip_parts[2]}.{ip_parts[3]}"

    for i, d in enumerate(devices_to_create):
        ip = next_ip()
        mid = model_map.get(d["model"])
        if mid is None:
            print(f"  ✗ {d['name']}: 型号 {d['model']} 不存在")
            continue
        days_ago = (i % 3 + 1) * 90
        payload = {
            "name": d["name"],
            "device_model_id": mid,
            "rack_id": None,
            "start_u": None,
            "end_u": None,
            "ip_addresses": ip,
            "serial_no": d["serial"],
            "asset_no": d["asset"],
            "department": d["dept"],
            "owner": d["owner"],
            "function": d["name"].split("-")[0],
            "purchase_date": str(date.today() - timedelta(days=days_ago)),
            "warranty_expire": str(date.today() + timedelta(days=365 * 3 - days_ago)),
            "status": "online" if i % 3 != 2 else "offline",
            "power_watt": 0,
        }
        resp = requests.post(f"{BASE}/devices/", json=payload)
        if resp.status_code == 201:
            print(f"  ✓ {d['name']} → {ip} ({resp.json()['status']})")
        else:
            print(f"  ✗ {d['name']}: {resp.status_code} {resp.text[:100]}")

    print(f"\n=== 3. 完成 ===")
    rc = requests.get(f"{BASE}/devices/")
    rl = requests.get(f"{BASE}/device-models/")
    print(f"  型号: {len(rl.json())} 个, 设备: {len(rc.json())} 台")

if __name__ == "__main__":
    main()
