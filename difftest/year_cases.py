import json, sys
from datetime import datetime

def ms(iso):
    return int(datetime.fromisoformat(iso.replace("Z", "+00:00")).timestamp() * 1000)

phrases = []
for y in [1, 99, 100, 500, 1800, 1883, 1900, 1918, 1970, 2007, 2026, 2037, 2038, 2041, 2050, 2099, 2100, 2150, 2400, 3000, 8545, 9999]:
    for t in ["Oct 1 {y} at 5pm", "July 4 {y} 3pm", "March 8 {y} 2:30am", "March 14 {y} 2:30am", "March 29 {y} 2:30am", "Nov 1 {y} 1:30am", "Nov 7 {y} 1:30 am", "Oct 25 {y} 2:30am", "{y}-03-10T02:30", "{y}-11-03T01:30:00", "3 days after Oct 1 {y}", "Dec 31 {y} 11pm", "Jan 1 {y}", "{y} BE", "{y} BC", "{y} AD", "Mar {y}"]:
        phrases.append(t.format(y=y))
phrases += ["2:30am", "1:30am", "tomorrow 2:30am", "next sunday 2:30am", "in 1 day", "in 24 hours", "in 1 week at 2:30am", "last sunday 1:30am", "midnight", "tonight", "last night", "this week", "this month", "this year", "now", "3 days ago 2:30am"]
refs = ["2026-03-08T06:59:00Z", "2026-03-08T07:30:00Z", "2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z", "2026-03-29T00:30:00Z", "2026-03-29T01:30:00Z", "2026-10-25T00:30:00Z", "2026-10-25T01:30:00Z", "2041-03-10T07:30:00Z", "2150-07-01T12:00:00Z"]
zones = ["UTC", "America/New_York", "Europe/London", "Europe/Berlin", "Australia/Sydney", "Asia/Kolkata", "America/Los_Angeles", "Pacific/Auckland"]
n = 0
with open(sys.argv[1], "w") as f:
    for r in refs:
        for z in zones:
            for tz in [None, -300]:
                for fw in [False, True]:
                    for p in phrases:
                        f.write(json.dumps({"id": n, "setup": f"{r}|{z}|{tz}|{fw}", "source": "years-dst", "text": p, "ref": ms(r), "tz": tz, "zone": z, "forward": fw, "config": "casual"}) + "\n")
                        n += 1
print(n)
