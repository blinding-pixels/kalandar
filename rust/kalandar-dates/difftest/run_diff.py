import json, os, subprocess, sys, collections

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
CHRONO_PATH = os.environ["CHRONO_PATH"]
RUST_BIN = os.path.join(CRATE, "..", "target", "release", "examples", "diff_runner")

def run(cmd, cases, env=None):
    data = "".join(json.dumps(c) + "\n" for c in cases)
    p = subprocess.run(cmd, input=data, capture_output=True, text=True, env=env)
    if p.returncode != 0:
        sys.stderr.write(p.stderr)
        raise SystemExit(f"{cmd[0]} failed")
    return {o["id"]: o["results"] for o in map(json.loads, p.stdout.splitlines())}

def main(cases_path, out_path):
    cases = [json.loads(l) for l in open(cases_path) if l.strip()]
    for i, c in enumerate(cases):
        c.setdefault("id", i)
    by_zone = collections.defaultdict(list)
    for c in cases:
        by_zone[c.get("zone", "UTC")].append(c)
    node_out, rust_out = {}, {}
    for zone, group in by_zone.items():
        env = dict(os.environ, TZ=zone, CHRONO_PATH=CHRONO_PATH)
        node_out.update(run(["node", os.path.join(HERE, "node_runner.mjs")], group, env))
        rust_out.update(run([RUST_BIN], group))
    diffs = []
    for c in cases:
        a, b = node_out.get(c["id"]), rust_out.get(c["id"])
        if a != b:
            diffs.append({"case": c, "node": a, "rust": b})
    with open(out_path, "w") as f:
        for d in diffs:
            f.write(json.dumps(d) + "\n")
    with_results = sum(1 for c in cases if node_out.get(c["id"]))
    print(f"cases={len(cases)} node_nonempty={with_results} diffs={len(diffs)}")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
