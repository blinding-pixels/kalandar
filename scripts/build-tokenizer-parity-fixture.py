#!/usr/bin/env python3
"""Build tokenizer_hf_parity.json for the Rust, JavaScript and Swift tests.

Tokenizes authored stress strings (accents, non-Latin scripts, emoji, symbols,
control and format characters, CJK, long words) and optional extra lines with
the Hugging Face BertTokenizerFast the capture models were trained with, so the
JavaScript, Rust and Swift tokenizers can check they produce the same ids.
Requires transformers.

python3 scripts/build-tokenizer-parity-fixture.py [--extra lines.json]
"""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path

from transformers import BertTokenizerFast

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
VOCAB_DIR = PACKAGE_ROOT / "resources" / "capture-parser" / "task"
OUTPUTS = [
    PACKAGE_ROOT / "rust" / "kalandar" / "tests" / "fixtures" / "tokenizer_hf_parity.json",
    PACKAGE_ROOT / "src" / "runtime" / "__tests__" / "fixtures" / "tokenizer_hf_parity.json",
    PACKAGE_ROOT / "swift" / "Tests" / "KalandarCoreAIRuntimeTests" / "Fixtures" / "tokenizer_hf_parity.json",
]
MAX_TOKENS = 128

STRESS = [
    "café naïve résumé Zoë crème brûlée",
    "नमस्ते दुनिया कल सुबह",
    "مرحبا غدا الساعة ٣",
    "สวัสดีพรุ่งนี้",
    "내일 오전 회의",
    "明日の会議 午後3時",
    "明天下午三点开会",
    "meet 😀 at 3",
    "family 👨‍👩‍👧 dinner",
    "flag 🇮🇳 day",
    "™ ® © ° € £ ¥ ₹ § ¶",
    "½ ¼ ² ³ × ÷ ± ∞ ≠ ≤",
    "ｆｕｌｌｗｉｄｔｈ　ｔｅｘｔ",
    "e\u0301clair a\u0308",
    "zero\u200bwidth soft\u00adhyphen nb\u00a0sp",
    "tab\there\nnew",
    "bom\ufeffstart",
    "rtl \u202eoverride",
    "İstanbul ǅ ß ẞ ﬁ",
    "Ωmega ΣΑΣ σς",
    "Привет завтра",
    "გამარჯობა",
    "Ελληνικά",
    "ⓐⓑⓒ ①②③",
    "…—–‘’“”«»",
    "x\u0000y",
    "a\ufffdb",
    "q3 kick-off @ 9:30am #ops-team (re: deck) [draft] {v2} <tbd> ~soon~ ^up^ |pipe| \\slash/",
    "a" + "b" * 120 + " end",
    "a\u0085b a\u2028b a\ue000b a\U000e0001b",
    "x\U0002b820y \U0002b920 \U0002f800z",
    "😀😀 👍🏽 a😀b",
    "\U0001d400\U0001d401 math",
    "Ⅻ ⅻ",
    "ﬀ ﬃ",
    "a\u0300\u0301\u0302b",
    " ".join(["then sync with the design team about the roadmap"] * 20),
]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--extra", type=Path, help="JSON list of extra lines; 300 are sampled")
    args = parser.parse_args()
    texts = list(STRESS)
    if args.extra:
        extra = json.loads(args.extra.read_text(encoding="utf-8"))
        random.Random(7).shuffle(extra)
        texts.extend(extra[:300])
    tokenizer = BertTokenizerFast.from_pretrained(str(VOCAB_DIR))
    cases = [
        {"text": text, "input_ids": tokenizer(text, truncation=True, max_length=MAX_TOKENS)["input_ids"]}
        for text in texts
    ]
    payload = {"vocab": "resources/capture-parser/task/vocab.txt", "max_sequence_length": MAX_TOKENS, "cases": cases}
    for output in OUTPUTS:
        output.write_text(json.dumps(payload, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"wrote {len(cases)} cases to {output.relative_to(PACKAGE_ROOT)}")


if __name__ == "__main__":
    main()
