import json, os, subprocess, sys, collections

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)

def norm(v):
    if isinstance(v, list):
        return sorted(norm(x) for x in v) if all(not isinstance(x, (list, dict)) for x in v) else v
    return v

def main(path):
    superseded = json.load(open(os.path.join(HERE, "superseded.json")))
    rows = [json.loads(l) for l in open(path) if l.strip()]
    skipped = [r["id"] for r in rows if r["id"] in superseded]
    rows = [r for r in rows if r["id"] not in superseded]
    if skipped:
        print(f"skipped {len(skipped)} superseded rows")
    feed = "".join(json.dumps(r) + "\n" for r in rows)
    out = subprocess.run([os.path.join(CRATE, "target", "release", "examples", "rows_eval")], input=feed, capture_output=True, text=True, check=True).stdout
    preds = {o["id"]: o for o in map(json.loads, out.splitlines())}
    stats = collections.defaultdict(lambda: [0, 0])
    failures = []
    for r in rows:
        p = preds[r["id"]]
        problems = []
        for key, want in r["expect"].items():
            if key == "recurrence":
                got = p["recurrence"] or {}
                if want is None and p["recurrence"] is not None:
                    problems.append(("recurrence", p["recurrence"]["rrule"], None))
                    continue
                for k, v in (want or {}).items():
                    g = got.get(k)
                    if k == "weekdays":
                        g = g or []
                    if norm(g) != norm(v):
                        problems.append((f"recurrence.{k}", g, v))
            elif key == "absent_warnings":
                present = [w for w in want if w in p["warnings"]]
                if present:
                    problems.append(("absent_warnings", p["warnings"], present))
            elif key == "warnings":
                missing = [w for w in want if w not in p["warnings"]]
                if missing:
                    problems.append(("warnings", p["warnings"], missing))
            else:
                if p.get(key) != want:
                    problems.append((key, p.get(key), want))
        stats[r["category"]][0] += 1
        stats[r["category"]][1] += not problems
        if problems:
            failures.append({"id": r["id"], "text": r["text"], "now": r.get("now"), "problems": problems})
    total = [0, 0]
    for cat, (n, ok) in sorted(stats.items()):
        total[0] += n; total[1] += ok
        print(f"{cat:16} {ok}/{n}")
    print(f"{'total':16} {total[1]}/{total[0]}")
    json.dump(failures, open(os.path.join(HERE, os.path.basename(path).replace('.jsonl', '_failures.json')), "w"), indent=1, ensure_ascii=False)

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "phase3_rows.jsonl"))
