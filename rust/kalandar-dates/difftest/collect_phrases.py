import glob, itertools, json, os, re, sys

MODEL_EVAL = os.environ.get("DIFFTEST_MODEL_EVAL", "")
SITE = os.environ.get("DIFFTEST_SITE", "")
NON_TIME_LABELS = {"title", "assignee", "project", "tags", "priority", "status", "location", "context_hint", "O"}

phrases = {}

def add(text, source):
    if not isinstance(text, str):
        return
    text = text.strip()
    if not text or len(text) > 400:
        return
    phrases.setdefault(text, source)

def spans_of(record):
    for key in ("entities", "spans", "subword_spans"):
        for span in record.get(key) or []:
            if isinstance(span, dict) and span.get("label") not in NON_TIME_LABELS:
                yield span

def add_record(record, source):
    add(record.get("text"), source + ":line")
    spans = [s for s in spans_of(record) if isinstance(s.get("text"), str)]
    for s in spans:
        add(s["text"], source + ":span")
    positioned = sorted((s for s in spans if isinstance(s.get("start"), int)), key=lambda s: s["start"])
    text = record.get("text") or ""
    for a, b in zip(positioned, positioned[1:]):
        if 0 <= b["start"] - a["end"] <= 1 and a.get("label") == b.get("label"):
            add(text[a["start"]:b["end"]], source + ":joined")
    if positioned:
        add(text[positioned[0]["start"]:positioned[-1]["end"]], source + ":all_time_spans")

for path in sorted(glob.glob(f"{MODEL_EVAL}/testset_*.jsonl") + glob.glob(f"{MODEL_EVAL}/labelled_*.jsonl") + glob.glob(f"{MODEL_EVAL}/outputs/*.jsonl")):
    source = os.path.basename(path)
    for line in open(path):
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            add_record(record, source)
            fields = record.get("fields") or {}
            for k, v in fields.items():
                if k not in NON_TIME_LABELS and isinstance(v, str) and not re.match(r"^\d{4}-\d{2}-\d{2}", v):
                    add(v, source + ":field")

resolved = json.load(open(f"{MODEL_EVAL}/outputs/resolved.json"))
for group in resolved.values():
    if isinstance(group, dict):
        for d in group.get("differences", []):
            add(d.get("text"), "resolved.json")

for path in glob.glob(f"{SITE}/vendor/kalandar/*/fixtures/*.json"):
    data = json.load(open(path))
    def walk(node):
        if isinstance(node, dict):
            for k, v in node.items():
                if k in ("text", "raw") and isinstance(v, str):
                    add(v, "kalandar:" + os.path.basename(path))
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)
    walk(data.get("cases", data))

for path in glob.glob(f"{SITE}/test/*.ts"):
    src = open(path).read()
    for m in re.finditer(r"(['\"`])((?:\\.|(?!\1).){3,300})\1", src):
        s = m.group(2)
        if "${" in s or "/" in s and " " not in s:
            continue
        if re.search(r"\s", s) or re.search(r"\d", s):
            add(s.encode().decode("unicode_escape") if "\\u" in s else s, "site-test:" + os.path.basename(path))

WEEKDAYS = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday", "Mon", "Tue", "Wed", "Thu", "Thurs", "Fri", "Sat", "Sun", "weekend", "weekday"]
MODIFIERS = ["", "this ", "next ", "last ", "past ", "on ", "on next ", "this coming "]
TIMES = ["", " at 5", " at 5pm", " at 5 pm", " 5PM", " at 17:00", " at 5:30pm", " at 5:30 p.m.", " at noon", " 9am", " at 9:15 am", " in the morning", " at night", " at 11 at night", " evening", " 12am", " at 12", " at 7 o'clock", " from 2 to 4pm", " 10am-2pm"]
for wd, mod, t in itertools.product(WEEKDAYS, MODIFIERS, TIMES):
    add(f"{mod}{wd}{t}", "matrix:weekday")
for wd in WEEKDAYS[:7]:
    for post in [" of this week", " of next week", " next week", " last week", ", the 12th", " 12/7/2014", " Jan 13"]:
        add(f"{wd}{post}", "matrix:weekday-post")

def ordinal(n):
    if 10 <= n % 100 <= 20:
        return f"{n}th"
    return f"{n}{ {1: 'st', 2: 'nd', 3: 'rd'}.get(n % 10, 'th') }"

ORDINAL_WORDS = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "eleventh", "twelfth", "thirteenth", "twentieth", "twenty first", "twenty-second", "thirtieth", "thirty-first"]
for n in range(1, 32):
    o = ordinal(n)
    for tmpl in ["{o}", "the {o}", "on the {o}", "{o} of the month", "the {o} at 3pm", "{o} of March", "{o} March", "March {o}", "March {o}, 2027", "{o} Dec 2026", "by the {o}", "due the {o} at 9"]:
        add(tmpl.format(o=o), "matrix:ordinal")
for w in ORDINAL_WORDS:
    for tmpl in ["the {w} of June", "{w} of june 2027", "June {w}", "on the {w}"]:
        add(tmpl.format(w=w), "matrix:ordinal-word")

MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December", "Jan", "Feb", "Mar", "Apr", "Jun", "Jul", "Aug", "Sep", "Sept", "Oct", "Nov", "Dec", "Dec."]
for mo in MONTHS:
    for tmpl in ["{m}", "in {m}", "{m} 2027", "{m}, 2025", "{m} 5", "{m} 5th", "5 {m}", "5th of {m}", "{m} 5 2027", "{m} 31", "{m} 29, 2027", "{m} 12 - 15", "12-15 {m} 2027", "{m} 3 at 10am", "{m} 3 to {m} 8", "2027 {m}", "early {m}", "the end of {m}", "{m} 1st 10:30"]:
        add(tmpl.format(m=mo), "matrix:month")

UNITS = ["second", "seconds", "minute", "minutes", "min", "mins", "hour", "hours", "hr", "hrs", "day", "days", "week", "weeks", "month", "months", "quarter", "year", "years", "yr", "h", "d", "w", "m", "mo", "y"]
NUMBERS = ["1", "2", "3", "10", "a", "an", "one", "two", "five", "twelve", "a few", "several", "a couple of", "half an", "1.5", "0.5"]
for u in UNITS:
    for n in NUMBERS:
        for tmpl in ["in {n} {u}", "{n} {u} ago", "{n} {u} later", "{n} {u} from now", "within {n} {u}", "for {n} {u}", "+{n} {u}", "-{n} {u}", "{n} {u} before", "{n} {u} after"]:
            add(tmpl.format(n=n, u=u), "matrix:relative")
    for tmpl in ["next {u}", "last {u}", "this {u}", "past {u}", "after this {u}"]:
        add(tmpl.format(u=u), "matrix:relative-word")

CASUAL = ["now", "today", "tonight", "tomorrow", "tmr", "tmrw", "overmorrow", "yesterday", "last night", "this morning", "this afternoon", "this evening", "noon", "midday", "midnight", "tomorrow morning", "tomorrow night", "tomorrow at noon", "today at midnight", "yesterday evening", "tonight at 8", "tomorrow at 5pm", "tomorrow 9am", "today 3:30", "the day after tomorrow", "morning", "afternoon", "evening", "night", "Tomorrow at 10 AM", "TONIGHT"]
for c in CASUAL:
    add(c, "matrix:casual")

TIMES_ONLY = ["5", "at 5", "5pm", "5 pm", "5p", "5PM", "5:30", "05:30", "17:30", "17:30:45", "17:30:45.123", "1730", "at 1730", "5.30pm", "5.30", "at 5.30", "12am", "12pm", "12:00", "0:15", "24:00", "at 25", "9 a.m.", "9 o'clock", "at 9 oclock", "8-10pm", "8pm-10", "10pm - 1am", "9:00 to 17:00", "from 9 until 5", "11pm-3", "2am until 4am", "at 3 in the afternoon", "at 6 at night", "5:30 in the morning", "13:00 PST", "10am EST", "10:00 GMT+2", "10:00 UTC-05:00", "3pm (EST)", "9am CET", "9am ET", "noon PT"]
for t in TIMES_ONLY:
    add(t, "matrix:time")

FORMATS = ["2026-10-20", "2026-10-20T10:00", "2026-10-20T10:00:00Z", "2026-10-20T10:00:00+05:30", "2026-10-20T10:00:00.123-0400", "2026/10/20", "2026.10.20", "2026 Oct 20", "10/20", "10/20/2026", "20/10/2026", "10/20/26", "1/2", "13/2", "10.20.2026", "10-20-2026", "10/2026", "06/2005", "2026-13-01", "2026-02-30", "Feb 29 2027", "Feb 29 2028", "12/7/2014 at 10:30", "Sat, 12/7/2014", "Tuesday, January 13, 2012", "2012-08-10 to 2012-08-15", "Aug 10 - 15 2012", "10th - 15th August", "from the 8th to the 10th", "from the 8th until the 10th of next month", "Mon to Fri", "Monday - Wednesday", "Friday to Monday", "Dec 25 to Jan 3", "Jan 3 to Dec 25", "2 weeks before 2026-10-20", "3 days after next friday", "2026-10-20 +2 weeks", "next tuesday +10 days", "1 BE", "2569 BE", "500 BC", "1999 AD", "the 2nd", "the second", "wait a second", "may", "you may come", "in may", "May 5", "march", "jan", "sat", "sun", "mon", "Wed.", "1.12.12", "v1.2", "at 1.2", "12345", "2014", "in 2014", "88", "on 3/4 at 2", "Q4", "next qtr", "end of day", "eod", "by Friday", "before 5pm", "until Monday", "Friday afternoon", "Thursday evening at 6", "this Friday night", "every Monday at 9", "Mondays at 6"]
for f in FORMATS:
    add(f, "matrix:format")

out = [{"text": t, "source": s} for t, s in phrases.items()]
json.dump(out, open(sys.argv[1], "w"), indent=0, ensure_ascii=False)
from collections import Counter
print(len(out), Counter(s.split(":")[0] for _, s in phrases.items()))
