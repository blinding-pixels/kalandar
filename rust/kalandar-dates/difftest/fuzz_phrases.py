import json, random, sys

random.seed(int(sys.argv[3]) if len(sys.argv) > 3 else 7)
phrases = [p["text"] for p in json.load(open(sys.argv[1]))]
short = [p for p in phrases if len(p) < 40]
fillers = ["", " ", ", ", " - ", " to ", " at ", " on ", " from ", " until ", " and ", " then ", " before ", " after ", " (", ") ", "; ", " – ", " ", "  ", " of ", " T", ".", " +", " -", " GMT", " EST ", " Z "]
words = ["meet", "call Bob", "dentist", "lunch", "report due", "remind me", "the", "a", "an", "it", "may", "second", "sat", "sun", "wed", "mar", "jan", "am", "pm", "o'clock", "night", "morning", "Ü", "café", "日本", "😀", "#3", "v2.1", "3/4", "1.5", "45", "2024", "99", "ok?"]
out = []
seen = set()
while len(out) < int(sys.argv[2]):
    parts = []
    for _ in range(random.randint(2, 5)):
        r = random.random()
        if r < 0.55:
            parts.append(random.choice(short))
        elif r < 0.85:
            parts.append(random.choice(words))
        else:
            parts.append(str(random.randint(0, 2100)))
        parts.append(random.choice(fillers))
    text = "".join(parts).strip()
    if random.random() < 0.2:
        text = text.upper()
    if text and text not in seen:
        seen.add(text)
        out.append({"text": text, "source": "fuzz"})
json.dump(out, open(sys.argv[4] if len(sys.argv) > 4 else "/dev/stdout", "w"), ensure_ascii=False)
