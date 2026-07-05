#!/usr/bin/env python3
"""Build a local Rust capture-parser resource bundle for parity tests.

The original Rust resource bundle is not vendored in this package. This script
creates a deterministic local bundle under .build with smoke-test ONNX models
that satisfy the Rust tensor contract. The app-level Rust semantics still come
from the copied Rust lightweight parser and tiny intent model.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import sys
from pathlib import Path
from typing import Any


PACKAGE_ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = PACKAGE_ROOT if (PACKAGE_ROOT / "training").exists() else PACKAGE_ROOT.parent
DEFAULT_OUTPUT = WORKSPACE_ROOT / ".build" / "rust-parity" / "capture-parser"
DEFAULT_ORT_SOURCE = (
    WORKSPACE_ROOT
    / "training"
    / ".venv"
    / "lib"
    / "python3.13"
    / "site-packages"
    / "onnxruntime"
    / "capi"
    / "libonnxruntime.1.24.4.dylib"
)
DEFAULT_ORT_LINK_DIR = WORKSPACE_ROOT / ".build" / "onnxruntime" / "lib"
TRAINING_SITE_PACKAGES = (
    WORKSPACE_ROOT / "training" / ".venv" / "lib" / "python3.13" / "site-packages"
)

MANIFEST_VERSION = "kalandar-capture-parser-artifacts-v1"
SUITE_VERSION = "kalandar-capture-parser-rust-v1"
NORMALIZATION_VERSION = "kalandar-rust-parity-normalization-v1"
POSTPROCESS_VERSION = "kalandar-rust-lightweight-v1"
TOKENIZER = {
    "kind": "bert_wordpiece",
    "doLowerCase": True,
    "unkToken": "[UNK]",
    "clsToken": "[CLS]",
    "sepToken": "[SEP]",
    "padToken": "[PAD]",
    "maxSequenceLength": 128,
}

CALENDAR_LABELS = [
    "O",
    "B-title",
    "I-title",
    "B-start_time",
    "I-start_time",
    "B-end_time",
    "I-end_time",
    "B-recurrence_start",
    "I-recurrence_start",
    "B-recurrence_end",
    "I-recurrence_end",
    "B-recurrence_exception",
    "I-recurrence_exception",
    "B-duration",
    "I-duration",
    "B-location",
    "I-location",
    "B-recurrence",
    "I-recurrence",
    "B-description",
    "I-description",
]
TASK_LABELS = [
    "O",
    "B-title",
    "I-title",
    "B-due_date",
    "I-due_date",
    "B-due_time",
    "I-due_time",
    "B-priority",
    "I-priority",
    "B-assignee",
    "I-assignee",
    "B-project",
    "I-project",
    "B-tags",
    "I-tags",
    "B-duration",
    "I-duration",
    "B-status",
    "I-status",
    "B-recurrence",
    "I-recurrence",
    "B-recurrence_start",
    "I-recurrence_start",
    "B-recurrence_end",
    "I-recurrence_end",
    "B-recurrence_exception",
    "I-recurrence_exception",
]
REMINDER_LABELS = [
    "O",
    "B-title",
    "I-title",
    "B-trigger_time",
    "I-trigger_time",
    "B-recurrence",
    "I-recurrence",
    "B-recurrence_start",
    "I-recurrence_start",
    "B-recurrence_end",
    "I-recurrence_end",
    "B-recurrence_exception",
    "I-recurrence_exception",
    "B-recurrence_window",
    "I-recurrence_window",
    "B-location",
    "I-location",
    "B-lead_time",
    "I-lead_time",
]
TOKEN_CLASSIFIERS = {
    "task": TASK_LABELS,
    "reminder": REMINDER_LABELS,
    "calendar": CALENDAR_LABELS,
}
SPECIAL_TOKENS = ["[PAD]", "[UNK]", "[CLS]", "[SEP]", "[MASK]"]
CAPABILITY_SAMPLES = [
    "finish report tomorrow",
    "remind me to water plants tomorrow at 9",
    "lunch with sara tomorrow at noon",
    "Lunch with Sara.",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--ort-source", type=Path, default=DEFAULT_ORT_SOURCE)
    parser.add_argument("--ort-link-dir", type=Path, default=DEFAULT_ORT_LINK_DIR)
    return parser.parse_args()


def require_onnx():
    try:
        import onnx  # type: ignore

        return onnx
    except ModuleNotFoundError:
        if TRAINING_SITE_PACKAGES.exists():
            sys.path.insert(0, str(TRAINING_SITE_PACKAGES))
        try:
            import onnx  # type: ignore

            return onnx
        except Exception as error:  # pragma: no cover - diagnostic path
            raise SystemExit(
                "onnx is required to build the Rust parity bundle. Run this "
                "with /opt/homebrew/bin/python3.13 from this checkout."
            ) from error


def write_json(path: Path, payload: Any, *, compact: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if compact:
        text = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
    else:
        text = json.dumps(payload, ensure_ascii=False, indent=2)
    path.write_text(f"{text}\n", encoding="utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_record(root: Path, relative_path: str, logical_name: str) -> dict[str, Any]:
    path = root / relative_path
    return {
        "logicalName": logical_name,
        "path": relative_path,
        "sha256": sha256_file(path),
        "sizeBytes": path.stat().st_size,
    }


def artifact_id(bundle: dict[str, Any]) -> str:
    files = sorted(bundle["files"], key=lambda entry: entry["logicalName"])
    fingerprint = {
        "parser": bundle["parser"],
        "runtimeKind": bundle["runtimeKind"],
        "tensorContractVersion": bundle["tensorContractVersion"],
        "normalizationVersion": bundle["normalizationVersion"],
        "postprocessVersion": bundle["postprocessVersion"],
        "tokenizer": bundle["tokenizer"],
        "labels": bundle["labels"],
        "files": files,
    }
    payload = json.dumps(fingerprint, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return f"sha256:{hashlib.sha256(payload).hexdigest()}"


def load_tiny_intent_model() -> dict[str, Any]:
    source = (PACKAGE_ROOT / "src" / "intent" / "tiny-model.ts").read_text(encoding="utf-8")
    match = re.search(r"export const TINY_INTENT_MODEL = (.*) as const;\s*$", source, re.S)
    if not match:
        raise SystemExit("could not locate TINY_INTENT_MODEL in src/intent/tiny-model.ts")
    return json.loads(match.group(1))


def fixture_inputs() -> list[str]:
    inputs = list(CAPABILITY_SAMPLES)
    fixtures_root = PACKAGE_ROOT / "rust" / "kalandar-capture-parser" / "tests" / "fixtures" / "kalandar"
    for fixture_path in fixtures_root.glob("*/goldens.json"):
        for entry in json.loads(fixture_path.read_text(encoding="utf-8")):
            if "input" in entry:
                inputs.append(entry["input"])
    app_lock = PACKAGE_ROOT / "coreai" / "app-parity-lock.json"
    if app_lock.exists():
        for entry in json.loads(app_lock.read_text(encoding="utf-8")).get("cases", []):
            if "input" in entry:
                inputs.append(entry["input"])
    return inputs


def wordpiece_vocab() -> list[str]:
    seen = dict.fromkeys(SPECIAL_TOKENS)
    for value in fixture_inputs():
        for token in re.findall(r"[a-z0-9]+(?:'[a-z0-9]+)?|[^\w\s]", value.lower()):
            seen[token] = None
    return list(seen)


def write_token_classifier_files(root: Path, family: str, labels: list[str], onnx: Any) -> dict[str, Any]:
    bundle_root = root / family
    bundle_root.mkdir(parents=True, exist_ok=True)
    write_zero_logits_onnx(bundle_root / "model.onnx", len(labels), onnx)
    write_json(bundle_root / "config.json", label_config(labels))
    write_json(bundle_root / "tokenizer_config.json", tokenizer_config())
    write_json(bundle_root / "special_tokens_map.json", special_tokens_map())
    (bundle_root / "vocab.txt").write_text("\n".join(wordpiece_vocab()) + "\n", encoding="utf-8")

    files = [
        file_record(root, f"{family}/model.onnx", "model"),
        file_record(root, f"{family}/config.json", "config"),
        file_record(root, f"{family}/tokenizer_config.json", "tokenizer_config"),
        file_record(root, f"{family}/special_tokens_map.json", "special_tokens_map"),
        file_record(root, f"{family}/vocab.txt", "vocab"),
    ]
    bundle = {
        "parser": family,
        "runtimeKind": "bert-token-classifier",
        "artifactId": "",
        "tensorContractVersion": f"{family}-bert-token-classification-v1",
        "normalizationVersion": NORMALIZATION_VERSION,
        "postprocessVersion": POSTPROCESS_VERSION,
        "tokenizer": TOKENIZER,
        "labels": labels,
        "files": files,
    }
    bundle["artifactId"] = artifact_id(bundle)
    return bundle


def write_intent_files(root: Path) -> dict[str, Any]:
    bundle_root = root / "intent"
    bundle_root.mkdir(parents=True, exist_ok=True)
    model = load_tiny_intent_model()
    write_json(bundle_root / "model.json", model, compact=True)
    bundle = {
        "parser": "intent",
        "runtimeKind": "tiny-linear",
        "artifactId": "",
        "tensorContractVersion": None,
        "normalizationVersion": NORMALIZATION_VERSION,
        "postprocessVersion": POSTPROCESS_VERSION,
        "tokenizer": None,
        "labels": model["labels"],
        "files": [file_record(root, "intent/model.json", "model_json")],
    }
    bundle["artifactId"] = artifact_id(bundle)
    return bundle


def label_config(labels: list[str]) -> dict[str, Any]:
    return {
        "id2label": {str(index): label for index, label in enumerate(labels)},
        "label2id": {label: index for index, label in enumerate(labels)},
        "max_position_embeddings": TOKENIZER["maxSequenceLength"],
    }


def tokenizer_config() -> dict[str, Any]:
    return {
        "do_lower_case": TOKENIZER["doLowerCase"],
        "unk_token": TOKENIZER["unkToken"],
        "sep_token": TOKENIZER["sepToken"],
        "pad_token": TOKENIZER["padToken"],
        "cls_token": TOKENIZER["clsToken"],
        "mask_token": "[MASK]",
        "model_max_length": TOKENIZER["maxSequenceLength"],
    }


def special_tokens_map() -> dict[str, str]:
    return {
        "unk_token": TOKENIZER["unkToken"],
        "sep_token": TOKENIZER["sepToken"],
        "pad_token": TOKENIZER["padToken"],
        "cls_token": TOKENIZER["clsToken"],
        "mask_token": "[MASK]",
    }


def write_zero_logits_onnx(path: Path, label_count: int, onnx: Any) -> None:
    from onnx import TensorProto, helper  # type: ignore

    input_ids = helper.make_tensor_value_info(
        "input_ids",
        TensorProto.INT64,
        ["batch", "sequence"],
    )
    attention_mask = helper.make_tensor_value_info(
        "attention_mask",
        TensorProto.INT64,
        ["batch", "sequence"],
    )
    logits = helper.make_tensor_value_info(
        "logits",
        TensorProto.FLOAT,
        ["batch", "sequence", label_count],
    )
    nodes = [
        helper.make_node("Cast", ["input_ids"], ["input_ids_float"], to=TensorProto.FLOAT),
        helper.make_node("Cast", ["attention_mask"], ["attention_mask_float"], to=TensorProto.FLOAT),
        helper.make_node(
            "Constant",
            [],
            ["unsqueeze_axes"],
            value=helper.make_tensor("unsqueeze_axes", TensorProto.INT64, [1], [2]),
        ),
        helper.make_node("Unsqueeze", ["input_ids_float", "unsqueeze_axes"], ["ids_3d"]),
        helper.make_node("Unsqueeze", ["attention_mask_float", "unsqueeze_axes"], ["mask_3d"]),
        helper.make_node(
            "Constant",
            [],
            ["zero"],
            value=helper.make_tensor("zero", TensorProto.FLOAT, [1], [0.0]),
        ),
        helper.make_node("Mul", ["ids_3d", "zero"], ["ids_zero"]),
        helper.make_node("Mul", ["mask_3d", "zero"], ["mask_zero"]),
        helper.make_node("Add", ["ids_zero", "mask_zero"], ["zero_seed"]),
        helper.make_node("Shape", ["input_ids"], ["input_shape"]),
        helper.make_node(
            "Constant",
            [],
            ["label_shape"],
            value=helper.make_tensor("label_shape", TensorProto.INT64, [1], [label_count]),
        ),
        helper.make_node("Concat", ["input_shape", "label_shape"], ["target_shape"], axis=0),
        helper.make_node("Expand", ["zero_seed", "target_shape"], ["logits"]),
    ]
    graph = helper.make_graph(
        nodes,
        f"kalandar_{label_count}_label_zero_logits",
        [input_ids, attention_mask],
        [logits],
    )
    model = helper.make_model(
        graph,
        producer_name="kalandar-rust-parity",
        opset_imports=[helper.make_opsetid("", 13)],
    )
    model.ir_version = 8
    onnx.checker.check_model(model)
    onnx.save(model, path)


def prepare_ort_link_dir(source: Path, link_dir: Path) -> None:
    if not source.exists():
        raise SystemExit(f"ONNX Runtime dylib not found: {source}")
    ensure_build_path(link_dir)
    link_dir.mkdir(parents=True, exist_ok=True)
    for name in ["libonnxruntime.1.24.4.dylib", "libonnxruntime.dylib"]:
        destination = link_dir / name
        if destination.exists() or destination.is_symlink():
            destination.unlink()
        try:
            os.symlink(source, destination)
        except OSError:
            shutil.copy2(source, destination)


def build_bundle(output: Path, onnx: Any) -> str:
    ensure_build_path(output)
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True, exist_ok=True)

    bundles = [write_intent_files(output)]
    for family, labels in TOKEN_CLASSIFIERS.items():
        bundles.append(write_token_classifier_files(output, family, labels, onnx))

    manifest = {
        "manifestVersion": MANIFEST_VERSION,
        "suiteVersion": SUITE_VERSION,
        "bundles": bundles,
    }
    manifest_path = output / "manifest.json"
    write_json(manifest_path, manifest)
    return sha256_file(manifest_path)


def ensure_build_path(path: Path) -> None:
    build_root = (WORKSPACE_ROOT / ".build").resolve()
    resolved = path.resolve() if path.exists() else path.parent.resolve() / path.name
    if resolved == build_root or build_root not in resolved.parents:
        raise SystemExit(f"refusing to replace path outside .build: {path}")


def main() -> None:
    args = parse_args()
    onnx = require_onnx()
    prepare_ort_link_dir(args.ort_source, args.ort_link_dir)
    manifest_sha256 = build_bundle(args.output, onnx)
    env_payload = {
        "resourceRoot": str(args.output.resolve()),
        "manifestSha256": manifest_sha256,
        "ortLibPath": str(args.ort_link_dir.resolve()),
        "ortPreferDynamicLink": "1",
        "ortDylibPath": str((args.ort_link_dir / "libonnxruntime.1.24.4.dylib").resolve()),
    }
    write_json(args.output.parent / "env.json", env_payload)
    print(json.dumps(env_payload, indent=2))


if __name__ == "__main__":
    main()
