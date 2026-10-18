import json, sys
from datetime import datetime, timezone

def ms(iso):
    return int(datetime.fromisoformat(iso.replace("Z", "+00:00")).timestamp() * 1000)

SETUPS = [
    {"name": "A-utc-casual", "ref": ms("2026-10-18T13:00:00Z"), "tz": None, "zone": "UTC", "forward": False, "config": "casual"},
    {"name": "B-ny-offset-forward", "ref": ms("2026-10-18T13:00:00Z"), "tz": -240, "zone": "America/New_York", "forward": True, "config": "casual"},
    {"name": "C-ny-system-forward", "ref": ms("2026-03-27T14:00:00Z"), "tz": None, "zone": "America/New_York", "forward": True, "config": "casual"},
    {"name": "D-kolkata-offset", "ref": ms("2026-11-01T05:30:00Z"), "tz": 330, "zone": "Asia/Kolkata", "forward": False, "config": "casual"},
    {"name": "E-berlin-abbr-dst-edge", "ref": ms("2026-03-29T00:30:00Z"), "tz": "CET", "zone": "Europe/Berlin", "forward": True, "config": "casual"},
    {"name": "F-utc-strict", "ref": ms("2026-10-18T13:00:00Z"), "tz": None, "zone": "UTC", "forward": False, "config": "strict"},
    {"name": "G-london-gb", "ref": ms("2026-10-25T00:30:00Z"), "tz": None, "zone": "Europe/London", "forward": False, "config": "gb"},
]

phrases = json.load(open(sys.argv[1]))
with open(sys.argv[2], "w") as out:
    n = 0
    for setup in SETUPS:
        for p in phrases:
            case = {"id": n, "setup": setup["name"], "source": p["source"], "text": p["text"]}
            case.update({k: v for k, v in setup.items() if k != "name"})
            out.write(json.dumps(case, ensure_ascii=False) + "\n")
            n += 1
print(n)
