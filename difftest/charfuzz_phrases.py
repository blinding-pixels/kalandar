import json, random, sys

random.seed(5)
alphabet = list("0123456789 :./-,apmAPMtThsTh") + ["st", "nd", "rd", "th", "am", "pm", "jan", "mon", "sun", "at ", "on ", "to ", "in ", "ago", "next ", "last ", "this ", "week", "day", "hour", "min", "Z", "+", "GMT", "EST", "CET", "(", ")", "oclock", "noon", "night", "tomorrow", " ", "　", "：", "〜", "–"]
out = set()
while len(out) < 20000:
    out.add("".join(random.choice(alphabet) for _ in range(random.randint(2, 14))).strip())
json.dump([{"text": t, "source": "charfuzz"} for t in sorted(out) if t], open(sys.argv[1], "w"), ensure_ascii=False)
