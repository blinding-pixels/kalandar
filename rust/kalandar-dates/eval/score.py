import json, os, subprocess, sys, collections

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
EVAL = os.environ.get("KALANDAR_MODEL_EVAL", "")
TIME_LABELS = {"start_time", "end_time", "duration", "due_date", "due_time", "trigger_time", "lead_time", "recurrence", "recurrence_start", "recurrence_end", "recurrence_exception", "recurrence_window"}
REC_FIELDS = ["frequency", "interval", "weekdays", "month_days", "months", "count", "start_date", "end_date"]

def load(path):
    return [json.loads(l) for l in open(path) if l.strip()]

def heldout_cases():
    golds = {g["id"]: g for g in load(f"{HERE}/gold_heldout.jsonl")}
    return [{"id": c["id"], "kind": c["kind"], "text": c["text"], "entities": c["entities"], "model_spans": [], "gold": golds[c["id"]]}
            for c in load(f"{HERE}/heldout_input.jsonl")]

def cases():
    out = []
    for kind in ["calendar", "reminder", "task"]:
        tests = load(f"{EVAL}/testset_{kind}.jsonl")
        models = {m["id"]: m for m in load(f"{EVAL}/outputs/model_test_{kind}.jsonl")}
        golds = {g["id"]: g for g in load(f"{HERE}/gold_{kind}.jsonl")}
        for t in tests:
            out.append({"id": t["id"], "kind": kind, "text": t["text"], "entities": t["entities"],
                        "model_spans": models.get(t["id"], {}).get("spans", []), "gold": golds[t["id"]]})
    return out

def norm_rec(r):
    if r is None:
        return None
    return {k: (sorted(r.get(k) or []) if isinstance(r.get(k), list) else r.get(k)) for k in REC_FIELDS}

def compare(pred, gold, kind):
    problems = []
    if pred["start"] != gold["start"]:
        problems.append(("start", pred["start"], gold["start"]))
    if kind == "calendar" and pred["end"] != gold["end"]:
        problems.append(("end", pred["end"], gold["end"]))
    pr, gr = norm_rec(pred["recurrence"]), norm_rec(gold["recurrence"])
    if (pr is None) != (gr is None):
        problems.append(("recurrence", pr, gr))
    elif pr and gr:
        for k in REC_FIELDS:
            if pr[k] != gr[k]:
                problems.append((f"recurrence.{k}", pr[k], gr[k]))
    if pred["lead_minutes"] != gold["lead_minutes"]:
        problems.append(("lead_minutes", pred["lead_minutes"], gold["lead_minutes"]))
    if bool(pred["alerts_off"]) != bool(gold["alerts_off"]):
        problems.append(("alerts_off", pred["alerts_off"], gold["alerts_off"]))
    if (pred["alert_at"] or None) != (gold.get("alert_at") or None):
        problems.append(("alert_at", pred["alert_at"], gold.get("alert_at")))
    if bool(pred["task_signal"]) != bool(gold["task_signal"]):
        problems.append(("task_signal", pred["task_signal"], gold["task_signal"]))
    return problems

def main():
    heldout = "--heldout" in sys.argv
    all_cases = heldout_cases() if heldout else cases()
    feed = "".join(json.dumps({k: v for k, v in c.items() if k != "gold"}) + "\n" for c in all_cases)
    binary = os.path.join(CRATE, "..", "target", "release", "examples", "capture_eval")
    out = subprocess.run([binary], input=feed, capture_output=True, text=True, check=True).stdout
    preds = {o["id"]: o for o in map(json.loads, out.splitlines())}
    report = {}
    failures = collections.defaultdict(list)
    modes = ["gold_spans", "no_spans"] if heldout else ["gold_spans", "model_spans", "model_capture", "no_spans"]
    for mode in modes:
        stats = collections.Counter()
        for c in all_cases:
            gold = c["gold"]
            has_time = any(e["label"] in TIME_LABELS for e in c["entities"]) or gold["start"] is not None
            if not has_time:
                continue
            kind = c["kind"]
            pred = preds[c["id"]][mode]
            problems = compare(pred, gold, kind)
            stats[(kind, "lines")] += 1
            stats[(kind, "exact")] += not problems
            stats[(kind, "start_ok")] += pred["start"] == gold["start"]
            if gold["recurrence"] is not None:
                stats[(kind, "rec_lines")] += 1
                stats[(kind, "rec_ok")] += norm_rec(pred["recurrence"]) == norm_rec(gold["recurrence"])
            if gold["ambiguous"]:
                stats[(kind, "ambiguous")] += 1
            if problems:
                failures[mode].append({"id": c["id"], "text": c["text"], "problems": problems, "notes": gold["notes"], "ambiguous": gold["ambiguous"], "warnings": pred["warnings"]})
        report[mode] = {k: {m: stats[(k, m)] for m in ["lines", "exact", "start_ok", "rec_lines", "rec_ok", "ambiguous"]} for k in ["calendar", "reminder", "task"]}
    json.dump({"summary": report, "failures": failures}, open(os.path.join(HERE, "score_heldout.json" if heldout else "score.json"), "w"), indent=1, ensure_ascii=False)
    for mode, per in report.items():
        tot = collections.Counter()
        for k, v in per.items():
            tot.update(v)
            print(f"{mode:12} {k:9} lines={v['lines']:3} exact={v['exact']:3} start_ok={v['start_ok']:3} recurrence={v['rec_ok']}/{v['rec_lines']}")
        print(f"{mode:12} {'total':9} lines={tot['lines']:3} exact={tot['exact']:3} start_ok={tot['start_ok']:3} recurrence={tot['rec_ok']}/{tot['rec_lines']}")

if __name__ == "__main__":
    main()
