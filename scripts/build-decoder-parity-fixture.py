#!/usr/bin/env python3
"""Build rust/kalandar/tests/fixtures/repair_model_parity.json.

Runs the round-two reference decode (Hugging Face tokenizer with
is_split_into_words=True, ONNX Runtime on the bundle model.onnx, first-piece
argmax and bio_decoder.decode) over a small set of authored sentences, so the
Rust runtime can check that its tokenizer, ONNX session and decoder reproduce
the evaluated word labels. Requires numpy, onnxruntime and transformers.

python3 scripts/build-decoder-parity-fixture.py \
  --bundles /path/to/bundles/repair --decoder-source /path/to/training/source
"""

from __future__ import annotations

import argparse
import os
import json
import re
import sys
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TRAINING = Path(os.environ.get("KALANDAR_TRAINING_OUTPUT", "."))
DEFAULT_OUTPUT = PACKAGE_ROOT / "rust" / "kalandar" / "tests" / "fixtures" / "repair_model_parity.json"
MAX_TOKENS = 128
DECODING = {"task": "argmax", "calendar": "bio", "reminder": "argmax"}
LONG_TAIL = " ".join(["then sync with the design team about the roadmap and the open questions"] * 12)

SENTENCES = {
    "task": [
        "finish the quarterly report by friday 5pm #finance high priority",
        "email Sara about the Atlas budget tomorrow morning",
        "call the plumber every monday at 9am except holidays",
        "review PR #42 for project Apollo, assign to Omar",
        "todo: buy groceries (milk, eggs) tonight",
        "pay rent on the 1st of every month starting next month",
        "draft launch plan in 90 minutes - status in progress",
        "remember to renew passport before   June 30th",
        f"prepare the board deck by monday {LONG_TAIL}",
    ],
    "calendar": [
        "lunch with Sara tomorrow at noon at Blue Bottle",
        "team standup every weekday 9:30-9:45am on Zoom",
        "dentist appointment next Tuesday 3pm for an hour",
        "dinner with mom and dad friday 7pm at Luigi's, bring wine",
        "quarterly planning offsite Oct 21-23 in Lisbon",
        "yoga class every Wednesday at 6pm until December",
        "1:1 with Priya at 4pm, discuss promotion",
        "flight to NYC on Nov 3 at 7:45am",
        f"planning session thursday 2pm at the office {LONG_TAIL}",
    ],
    "reminder": [
        "remind me to water the plants tomorrow at 9",
        "remind me to take my meds every day at 8am and 8pm",
        "remind me when I get to the office to call Phil",
        "remind me 15 minutes before the meeting to print slides",
        "ping me every other saturday to clean the gutters except in winter",
        "remind me to submit timesheet every friday at 4pm for 3 months",
        "remember to pick up dry cleaning at 6",
        "remind me tonight to text Jess happy birthday",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundles", type=Path, default=DEFAULT_TRAINING / "bundles" / "repair")
    parser.add_argument("--decoder-source", type=Path, default=DEFAULT_TRAINING / "source")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def spans(text: str, labels: list[str]) -> list[dict[str, object]]:
    matches = list(re.finditer(r"\S+", text))
    output: list[dict[str, object]] = []
    for index, label in enumerate(labels):
        if label == "O":
            continue
        prefix, field = label.split("-", 1)
        previous = labels[index - 1] if index > 0 else "O"
        if prefix == "B" or previous == "O" or previous[2:] != field:
            output.append({"label": field, "start": matches[index].start(), "end": matches[index].end()})
        else:
            output[-1]["end"] = matches[index].end()
    for span in output:
        span["text"] = text[span["start"] : span["end"]]
    return output


def main() -> int:
    args = parse_args()
    sys.path.insert(0, str(args.decoder_source))
    import numpy as np
    import onnxruntime as ort
    from transformers import AutoTokenizer
    from bio_decoder import decode

    cases = []
    for family, sentences in SENTENCES.items():
        bundle = args.bundles / family
        config = json.loads((bundle / "config.json").read_text(encoding="utf-8"))
        labels = [config["id2label"][str(index)] for index in range(len(config["id2label"]))]
        tokenizer = AutoTokenizer.from_pretrained(str(bundle))
        options = ort.SessionOptions()
        options.intra_op_num_threads = 1
        session = ort.InferenceSession(str(bundle / "model.onnx"), options, providers=["CPUExecutionProvider"])
        for text in sentences:
            words = text.split()
            encoded = tokenizer(words, is_split_into_words=True, truncation=True, max_length=MAX_TOKENS)
            feeds = {
                "input_ids": np.asarray([encoded["input_ids"]], dtype=np.int64),
                "attention_mask": np.asarray([encoded["attention_mask"]], dtype=np.int64),
            }
            logits = session.run(["logits"], feeds)[0][0]
            indices, word_ids, seen = [], [], set()
            for token_index, word_id in enumerate(encoded.word_ids()):
                if word_id is not None and word_id not in seen:
                    indices.append(token_index)
                    word_ids.append(word_id)
                    seen.add(word_id)
            first = logits[indices]
            decoded = {}
            for mode in ("argmax", "bio"):
                word_labels = ["O"] * len(words)
                for word_id, label_id in zip(word_ids, decode(first, labels, mode == "bio")):
                    word_labels[word_id] = labels[label_id]
                decoded[mode] = word_labels
            decoding = DECODING[family]
            cases.append(
                {
                    "family": family,
                    "decoding": decoding,
                    "text": text,
                    "represented_words": len(word_ids),
                    "argmax": decoded["argmax"],
                    "bio": decoded["bio"],
                    "spans": spans(text, decoded[decoding]),
                }
            )

    args.output.write_text(json.dumps(cases, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(args.output), "cases": len(cases)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
