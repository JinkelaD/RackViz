import subprocess
import re
import platform
from typing import Optional
from ..core.config import settings

def ping(ip: str) -> Optional[float]:
    param = "-n" if platform.system().lower() == "windows" else "-c"
    timeout_param = "-w" if platform.system().lower() == "windows" else "-W"
    count = str(settings.icmp_count)
    timeout = str(settings.icmp_timeout)

    try:
        result = subprocess.run(
            ["ping", param, count, timeout_param, timeout, ip],
            capture_output=True, text=True, timeout=settings.icmp_timeout + 2,
        )
        if result.returncode != 0:
            return None
        match = re.search(r"(?:time|时间)[=<](\d+\.?\d*)\s*ms", result.stdout)
        if match:
            return float(match.group(1))
        return 0.0
    except (subprocess.TimeoutExpired, Exception):
        return None