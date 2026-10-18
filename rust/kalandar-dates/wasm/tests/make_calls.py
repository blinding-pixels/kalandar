import json, os, sys

EVAL = os.environ.get("KALANDAR_MODEL_EVAL", "")
HERE = sys.argv[1]
calls = []
settings_variants = [None, {"clock": "24h", "weekStart": "sun", "weekend": ["fri", "sat"], "morning": "07:30", "dinner": "19:30", "defaultEventMinutes": 30, "dayFirst": True, "defaultNotificationTime": "07:45", "locale": "en-GB"}, {"holidays": "off"}, {"holidays": {"country": "IN"}}]
for kind in ["calendar", "reminder", "task"]:
    for i, line in enumerate(open(f"{EVAL}/testset_{kind}.jsonl")):
        r = json.loads(line)
        text = r["text"]
        def utf16(chars):
            return len(text[:chars].encode("utf-16-le")) // 2
        spans = [{"label": e["label"], "start": utf16(e["start"]), "end": utf16(e["end"])} for e in r["entities"]]
        base = {"line": text, "kind": kind, "now": "2026-10-18T17:00:00Z", "zone": "America/Los_Angeles", "spans": spans,
                "settings": settings_variants[i % len(settings_variants)]}
        for fn in ["resolveLine", "resolveCapture", "resolveUnlabelled"]:
            calls.append({"fn": fn, "request": base})
        calls.append({"fn": "parse", "request": {"text": text, "ref": 1792342800000, "zone": "Europe/Berlin", "tz": None, "forward": True, "config": "casual"}})
extra = ["buy Christmas lights", "sat with Ana", "lunch with June friday", "hourly from 10am to 4pm on weekdays", "pay cleaner on the 1st of every month, not in December", "family camping Fri to Sun", "call mom at 5 and pay rent on the 1st", "remind me after the meeting", "remind me every mon/wed/fri at 7am except holidays", "standup every weekday at 9:30 except Dec 25", "call 3pm EST Friday", "remind me 30 min before the 9am call Tuesday", "every 15 minutes between 9 and 5", "next Friday at 10", "0930 sync 03/04", "Christmas Eve dinner at 6", "every Monday for 6 weeks", "flight March 14 2027 at 2:30am", "café at quarter past 3 😀", "日本 meeting tomorrow 10am", "remind me every last Friday of the month", "Halloween costume ideas", "remind me to grab milk when I get home", "gym at 6 then dinner at 8", "water the ferns every so often", "send invoice once Sam signs off", "tidy the shed soonish", "skip Christmas, every Monday at 9", "diwali party", "thanksgiving dinner", "boxing day sale", "mothers day brunch", "eid al-fitr", "every holi", "standup every weekday at 9 except holidays"]
for text in extra:
    for kind in ["calendar", "reminder", "task"]:
        calls.append({"fn": "resolveUnlabelled", "request": {"line": text, "kind": kind, "now": "2026-10-18T17:00:00Z", "zone": "America/Los_Angeles"}})
with open(f"{HERE}/calls.jsonl", "w") as f:
    for c in calls:
        f.write(json.dumps(c, ensure_ascii=False) + "\n")
print(len(calls))
