import json
import os
import re
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
OUT_DIR = os.path.join(CRATE, "src", "tests")

COMPONENTS = {
    "year": "Year",
    "month": "Month",
    "day": "Day",
    "weekday": "Weekday",
    "hour": "Hour",
    "minute": "Minute",
    "second": "Second",
    "millisecond": "Millisecond",
    "meridiem": "Meridiem",
    "timezoneOffset": "TimezoneOffset",
}

SKIPPED_CONFIGS = {"fr": "uses the French locale (chrono.fr), outside the English-only scope"}


def rust_str(s):
    return json.dumps(s, ensure_ascii=False)


def snake(name):
    s = re.sub(r"[^A-Za-z0-9]+", "_", name).strip("_").lower()
    s = re.sub(r"^test_+", "", s)
    s = re.sub(r"_+", "_", s)
    return s or "case"


def reference_expr(ref):
    kind = ref["kind"]
    if kind == "now":
        return "now()"
    if kind == "date":
        return f"at({ref['instant']})"
    instant = ref.get("instant")
    base = "now()" if instant is None else f"at({instant})"
    tz = ref.get("timezone")
    if tz is None:
        return base
    if isinstance(tz, (int, float)):
        return f"{base}.with_timezone_offset({int(tz)})"
    return f"{base}.with_timezone_abbreviation({rust_str(tz)})"


def option_expr(option):
    parts = []
    if option.get("forwardDate"):
        parts.append(".forward()")
    for name, tz in (option.get("timezones") or {}).items():
        if "fixed" in tz:
            parts.append(f".timezone({rust_str(name)}, TimezoneAbbr::Fixed({tz['fixed']}))")
        else:
            parts.append(
                f".timezone({rust_str(name)}, TimezoneAbbr::Ambiguous {{ offset_during_dst: {tz['dst']}, offset_non_dst: {tz['nonDst']}, dst_start: {rule_expr(tz['start'])}, dst_end: {rule_expr(tz['end'])} }})"
            )
    return "options()" + "".join(parts)


WEEKDAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]


def rule_expr(rule):
    wd = f"Weekday::{WEEKDAYS[rule['weekday']]}"
    if rule["last"]:
        return f"DstRule::LastWeekdayOfMonth {{ month: {rule['month']}, weekday: {wd}, hour: {rule['hour']} }}"
    return f"DstRule::NthWeekdayOfMonth {{ month: {rule['month']}, weekday: {wd}, n: {rule['n']}, hour: {rule['hour']} }}"


class Untranslatable(Exception):
    pass


def components_expr(var, path):
    idx = int(path[0])
    base = f"{var}[{idx}]"
    if len(path) == 1:
        return base, "result"
    part = path[1]
    if part == "start":
        return f"{base}.start", "components"
    if part == "end":
        return f"{base}.end", "optional_components"
    raise Untranslatable(f"path {path}")


def value_expr(var, path, text):
    if not path:
        raise Untranslatable("empty path")
    head = path[0]
    if isinstance(head, dict) and head["call"] == "__count":
        return f"{var}.len()", "count"
    if head == "length":
        return f"{var}.len()", "count"
    idx = int(head)
    base = f"{var}[{idx}]"
    rest = path[1:]
    if not rest:
        return base, "result"
    part = rest[0]
    if part == "text" and len(rest) == 1:
        return f"{base}.text", "text"
    if part == "index" and len(rest) == 1:
        return f"index16({rust_str(text)}, &{base})", "index"
    target = None
    kind = None
    if part in ("start", "end"):
        target = f"{base}.start" if part == "start" else f"{base}.end"
        kind = "components" if part == "start" else "optional_components"
        rest = rest[1:]
    else:
        target = base
        kind = "result"
    if not rest:
        return target, kind
    if kind == "optional_components":
        target = f"{target}.as_ref().expect(\"end\")"
    call = rest[0]
    if not isinstance(call, dict):
        raise Untranslatable(f"path {path}")
    name = call["call"]
    if name == "get" and len(rest) == 1:
        return f"{target}.get(Component::{COMPONENTS[call['args'][0]]})", "component_value"
    if name == "isCertain" and len(rest) == 1:
        return f"{target}.is_certain(Component::{COMPONENTS[call['args'][0]]})", "bool"
    if name == "tags" and len(rest) == 1:
        return f"{target}.tags()", "tags"
    if name == "date":
        if len(rest) == 1:
            return f"millis({target}.date())", "millis"
        if len(rest) == 2 and isinstance(rest[1], dict) and rest[1]["call"] == "getTime":
            return f"millis({target}.date())", "millis"
    raise Untranslatable(f"path {path}")


def expected_scalar(expected):
    if isinstance(expected, dict):
        if "__date" in expected:
            return expected["__date"]
        if expected.get("__undefined"):
            return None
    return expected


def assertion_lines(event, var, text):
    matcher = event["matcher"]
    negate = event["not"]
    expected = expected_scalar(event.get("expected"))
    path = event["path"]
    if event.get("perf"):
        return ["__PERF__"]
    expr, kind = value_expr(var, path, text)

    if matcher == "toBeNull":
        if kind == "components":
            return []
        if kind == "optional_components":
            return [f"assert!({expr}.is_some());" if negate else f"assert!({expr}.is_none());"]
        raise Untranslatable(f"toBeNull on {kind}")
    if matcher in ("toBeTruthy", "toBeFalsy"):
        truthy = (matcher == "toBeTruthy") != negate
        if kind == "bool":
            return [f"assert!({expr});" if truthy else f"assert!(!{expr});"]
        if kind == "component_value":
            return [f"assert!(truthy({expr}));" if truthy else f"assert!(!truthy({expr}));"]
        raise Untranslatable(f"{matcher} on {kind}")
    if matcher == "toContain":
        if kind in ("tags", "text"):
            line = f"{expr}.contains({rust_str(expected)})"
            return [f"assert!(!{line});" if negate else f"assert!({line});"]
        raise Untranslatable(f"toContain on {kind}")
    if matcher == "toBeDate":
        if kind in ("components", "result"):
            target = expr
        elif kind == "optional_components":
            target = f"{expr}.as_ref().expect(\"end\")"
        else:
            raise Untranslatable(f"toBeDate on {kind}")
        op = "assert_ne" if negate else "assert_eq"
        return [f"{op}!(millis({target}.date()), Some({expected}));"]
    if matcher in ("toBe", "toEqual", "toStrictEqual", "toBeCloseTo"):
        op = "assert_ne" if negate else "assert_eq"
        if kind == "component_value":
            value = "None" if expected is None else f"Some({int(expected)})"
            return [f"{op}!({expr}, {value});"]
        if kind == "count":
            return [f"{op}!({expr}, {int(expected)});"]
        if kind == "index":
            return [f"{op}!({expr}, {int(expected)});"]
        if kind == "text":
            return [f"{op}!({expr}, {rust_str(expected)});"]
        if kind == "bool":
            truthy = bool(expected) != negate
            return [f"assert!({expr});" if truthy else f"assert!(!{expr});"]
        if kind == "millis":
            return [f"{op}!({expr}, Some({int(expected)}));"]
        raise Untranslatable(f"{matcher} on {kind}")
    raise Untranslatable(f"matcher {matcher}")


def main(recorded_path):
    data = json.load(open(recorded_path))
    by_file = defaultdict(list)
    for test in data["tests"]:
        by_file[test["file"]].append(test)

    os.makedirs(OUT_DIR, exist_ok=True)
    module_names = []
    unported = []
    stats = Counter()

    for file_name, tests in sorted(by_file.items()):
        module = snake(file_name.replace(".test.ts", ""))
        module_names.append(module)
        used_names = Counter()
        fns = []
        for test in tests:
            name = snake(test["name"])
            used_names[name] += 1
            if used_names[name] > 1:
                name = f"{name}_{used_names[name]}"
            parses = {}
            mutated = set(e["parse"] for e in test["events"] if e["type"] == "mutate")
            body = []
            skipped_parses = set()
            for event in test["events"]:
                etype = event["type"]
                if etype == "parse":
                    if event["config"] in SKIPPED_CONFIGS:
                        skipped_parses.add(event["id"])
                        unported.append((file_name, test["name"], f"case {event['text']!r} {SKIPPED_CONFIGS[event['config']]}"))
                        stats["skipped_parses"] += 1
                        continue
                    var = f"r{len(parses)}"
                    parses[event["id"]] = (var, event["text"])
                    mut = "mut " if event["id"] in mutated else ""
                    body.append(
                        f"let {mut}{var} = run({rust_str(event['config'])}, {rust_str(event['text'])}, {reference_expr(event['ref'])}, {option_expr(event['option'])});"
                    )
                    stats["parses"] += 1
                elif etype == "perf_begin":
                    body.append("let started = std::time::Instant::now();")
                elif etype == "perf_end":
                    body.append("let elapsed = started.elapsed().as_millis();")
                elif etype == "mutate":
                    if event["parse"] in skipped_parses:
                        continue
                    var, _ = parses[event["parse"]]
                    target, kind = components_expr(var, event["path"])
                    comp = COMPONENTS[event["args"][0]]
                    if kind == "optional_components":
                        target = f"{target}.as_mut().expect(\"end\")"
                    body.append(f"{target}.{event['method']}(Component::{comp}, {int(event['args'][1])});")
                    stats["mutations"] += 1
                elif etype == "assert":
                    if event.get("perf"):
                        body.append(f"assert!(elapsed < {int(event['expected'])});")
                        stats["assertions"] += 1
                        continue
                    if event["parse"] in skipped_parses:
                        stats["skipped_assertions"] += 1
                        continue
                    var, text = parses[event["parse"]]
                    try:
                        lines = assertion_lines(event, var, text)
                    except Untranslatable as e:
                        unported.append((file_name, test["name"], f"assertion not translatable: {e}"))
                        stats["untranslatable"] += 1
                        continue
                    if not lines:
                        stats["type_enforced"] += 1
                    else:
                        stats["assertions"] += 1
                    body.extend(lines)
            for reason in sorted(set(test["unportable"])):
                unported.append((file_name, test["name"], reason))
            if not body:
                continue
            for var, _ in parses.values():
                used = any(re.search(rf"\b{var}\b", l) for l in body if not l.startswith(f"let {var} ") and not l.startswith(f"let mut {var} "))
                if not used:
                    body = [l.replace(f"let {var} =", f"let _{var} =", 1) if l.startswith(f"let {var} =") else l for l in body]
            fns.append((name, body))

        lines = ["use super::support::*;", "use crate::types::Component;"]
        src = "\n".join(l for _, b in fns for l in b)
        if "TimezoneAbbr::" in src:
            lines.append("use crate::types::TimezoneAbbr;")
        if "DstRule::" in src:
            lines.append("use crate::types::{DstRule, Weekday};")
        if "Component::" not in src:
            lines.remove("use crate::types::Component;")
        lines.append("")
        for name, body in fns:
            lines.append("#[test]")
            lines.append(f"fn {name}() {{")
            for l in body:
                lines.append(f"    {l}")
            lines.append("}")
            lines.append("")
        with open(os.path.join(OUT_DIR, f"{module}.rs"), "w") as f:
            f.write("\n".join(lines).rstrip() + "\n")

    with open(os.path.join(OUT_DIR, "mod.rs"), "w") as f:
        f.write("mod support;\n\n")
        if os.path.exists(os.path.join(OUT_DIR, "hand_ported.rs")):
            f.write("mod hand_ported;\n")
        for m in module_names:
            f.write(f"mod {m};\n")

    with open(os.path.join(HERE, "unported.json"), "w") as f:
        json.dump([{"file": a, "test": b, "reason": c} for a, b, c in unported], f, indent=1)
    print(dict(stats))
    print(f"tests={len(data['tests'])} modules={len(module_names)} unported_items={len(unported)}")
    for item in unported:
        print("UNPORTED", *item)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "recorded.json"))
