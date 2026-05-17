#!/usr/bin/env python3
"""Validate device catalog JSON examples against local schema metadata."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "devices-catalog.schema.json"
EXAMPLE_DIR = ROOT / "examples"
LABEL_RE = re.compile(r"^[A-Za-z0-9:_./@-]+$")
COGNITIVE_CLASSES = {"accelerator", "memory", "model"}
DMA_CLASSES = {"block", "accelerator", "memory", "audit-storage"}


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def require_object(value: Any, path: str, errors: list[str]) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        errors.append(f"{path}: expected object")
        return None
    return value


def validate_label(value: Any, path: str, errors: list[str]) -> None:
    if not isinstance(value, str) or not value:
        errors.append(f"{path}: expected non-empty string")
        return
    if len(value) > 96:
        errors.append(f"{path}: label too long")
    if not LABEL_RE.fullmatch(value):
        errors.append(f"{path}: invalid label characters")


def validate_bool(value: Any, path: str, errors: list[str]) -> None:
    if not isinstance(value, bool):
        errors.append(f"{path}: expected boolean")


def validate_enum(value: Any, path: str, allowed: set[str], errors: list[str]) -> None:
    if not isinstance(value, str) or value not in allowed:
        errors.append(f"{path}: unsupported value {value!r}")


def validate_enum_list(
    value: Any,
    path: str,
    allowed: set[str],
    errors: list[str],
    *,
    require_non_empty: bool = True,
) -> set[str]:
    if not isinstance(value, list):
        errors.append(f"{path}: expected list")
        return set()
    if require_non_empty and not value:
        errors.append(f"{path}: expected non-empty list")
    seen: set[str] = set()
    for index, item in enumerate(value):
        item_path = f"{path}[{index}]"
        validate_enum(item, item_path, allowed, errors)
        if isinstance(item, str):
            if item in seen:
                errors.append(f"{item_path}: duplicate value")
            seen.add(item)
    return seen


def reject_unknown_keys(
    value: dict[str, Any], path: str, allowed_keys: set[str], errors: list[str]
) -> None:
    for key in value:
        if key not in allowed_keys:
            errors.append(f"{path}.{key}: unexpected field")


def require_keys(value: dict[str, Any], path: str, required_keys: set[str], errors: list[str]) -> None:
    for key in required_keys:
        if key not in value:
            errors.append(f"{path}.{key}: missing required field")


def validate_class(
    value: Any, path: str, metadata: dict[str, set[str] | str], errors: list[str]
) -> str | None:
    entry = require_object(value, path, errors)
    if entry is None:
        return None
    required = {"class", "operations", "capabilities", "open_rights", "mock_supported"}
    reject_unknown_keys(entry, path, required, errors)
    require_keys(entry, path, required, errors)
    validate_enum(entry.get("class"), f"{path}.class", metadata["classes"], errors)  # type: ignore[arg-type]
    operations = validate_enum_list(
        entry.get("operations"),
        f"{path}.operations",
        metadata["operations"],  # type: ignore[arg-type]
        errors,
    )
    capabilities = validate_enum_list(
        entry.get("capabilities"),
        f"{path}.capabilities",
        metadata["capabilities"],  # type: ignore[arg-type]
        errors,
    )
    rights = validate_enum_list(
        entry.get("open_rights"),
        f"{path}.open_rights",
        metadata["rights"],  # type: ignore[arg-type]
        errors,
    )
    validate_bool(entry.get("mock_supported"), f"{path}.mock_supported", errors)

    class_name = entry.get("class")
    if not isinstance(class_name, str):
        return None
    if "open" not in rights or "call" not in rights:
        errors.append(f"{path}.open_rights: device opens require open and call rights")
    if class_name in COGNITIVE_CLASSES and "cognitive" not in capabilities:
        errors.append(f"{path}.capabilities: cognitive class must declare cognitive capability")
    if class_name in COGNITIVE_CLASSES and "cognition" not in rights:
        errors.append(f"{path}.open_rights: cognitive class must require cognition")
    if class_name in DMA_CLASSES and "dma" not in capabilities:
        errors.append(f"{path}.capabilities: DMA class must declare dma capability")
    if class_name == "block" and not {"block_read", "block_write"}.issubset(operations):
        errors.append(f"{path}.operations: block class requires block_read and block_write")
    if class_name == "memory" and not {"put_record", "get_record"}.issubset(operations):
        errors.append(f"{path}.operations: memory class requires put_record and get_record")
    if class_name == "model" and "infer" not in operations:
        errors.append(f"{path}.operations: model class requires infer")
    if class_name == "accelerator" and "read_result" not in operations:
        errors.append(f"{path}.operations: accelerator class requires read_result")
    return class_name


def validate_root_list(
    manifest: dict[str, Any],
    key: str,
    metadata_key: str,
    metadata: dict[str, set[str] | str],
    errors: list[str],
) -> None:
    observed = validate_enum_list(
        manifest.get(key),
        f"devices-catalog.json.{key}",
        metadata[metadata_key],  # type: ignore[arg-type]
        errors,
    )
    expected = metadata[metadata_key]
    if isinstance(expected, set) and observed != expected:
        errors.append(f"devices-catalog.json.{key}: expected {sorted(expected)}")


def validate_manifest(path: Path, metadata: dict[str, set[str] | str], errors: list[str]) -> None:
    manifest = require_object(load_json(path), path.name, errors)
    if manifest is None:
        return
    required = {
        "schema_version",
        "repository",
        "version",
        "modules",
        "classes",
        "dma_policies",
        "interrupt_policies",
        "data_classes",
        "redaction_states",
    }
    reject_unknown_keys(manifest, path.name, required, errors)
    require_keys(manifest, path.name, required, errors)
    if manifest.get("schema_version") != metadata["schema_version"]:
        errors.append(f"{path.name}.schema_version: expected {metadata['schema_version']!r}")
    if manifest.get("repository") != metadata["repository"]:
        errors.append(f"{path.name}.repository: expected {metadata['repository']!r}")
    validate_label(manifest.get("version"), f"{path.name}.version", errors)

    modules = validate_enum_list(
        manifest.get("modules"), f"{path.name}.modules", metadata["modules"], errors  # type: ignore[arg-type]
    )
    if modules != metadata["modules"]:
        errors.append(f"{path.name}.modules: expected {sorted(metadata['modules'])}")

    classes_value = manifest.get("classes")
    if not isinstance(classes_value, list) or not classes_value:
        errors.append(f"{path.name}.classes: expected non-empty list")
    else:
        seen: set[str] = set()
        for index, class_entry in enumerate(classes_value):
            class_name = validate_class(class_entry, f"{path.name}.classes[{index}]", metadata, errors)
            if class_name is None:
                continue
            if class_name in seen:
                errors.append(f"{path.name}.classes[{index}].class: duplicate class")
            seen.add(class_name)
        if seen != metadata["classes"]:
            errors.append(f"{path.name}.classes: expected {sorted(metadata['classes'])}")

    validate_root_list(manifest, "dma_policies", "dma_policies", metadata, errors)
    validate_root_list(manifest, "interrupt_policies", "interrupt_policies", metadata, errors)
    validate_root_list(manifest, "data_classes", "data_classes", metadata, errors)
    validate_root_list(manifest, "redaction_states", "redaction_states", metadata, errors)


def schema_set(schema: dict[str, Any], key: str) -> set[str]:
    value = schema.get(key)
    if not isinstance(value, list):
        return set()
    return {item for item in value if isinstance(item, str)}


def main() -> int:
    schema = load_json(SCHEMA_PATH)
    metadata: dict[str, set[str] | str] = {
        "schema_version": schema.get("x-alani-schema-version", ""),
        "repository": schema.get("x-alani-repository", ""),
        "modules": schema_set(schema, "x-alani-modules"),
        "classes": schema_set(schema, "x-alani-device-classes"),
        "operations": schema_set(schema, "x-alani-operations"),
        "capabilities": schema_set(schema, "x-alani-capabilities"),
        "rights": schema_set(schema, "x-alani-rights"),
        "dma_policies": schema_set(schema, "x-alani-dma-policies"),
        "interrupt_policies": schema_set(schema, "x-alani-interrupt-policies"),
        "data_classes": schema_set(schema, "x-alani-data-classes"),
        "redaction_states": schema_set(schema, "x-alani-redaction-states"),
    }
    if metadata["schema_version"] != "alani.devices.v1":
        print("invalid devices schema metadata", file=sys.stderr)
        return 1
    if metadata["repository"] != "alani-devices":
        print("invalid devices repository metadata", file=sys.stderr)
        return 1
    if any(not value for key, value in metadata.items() if key not in {"schema_version", "repository"}):
        print("devices schema enum metadata is incomplete", file=sys.stderr)
        return 1

    examples = sorted(EXAMPLE_DIR.glob("*.json"))
    if not examples:
        print("no device JSON examples found", file=sys.stderr)
        return 1

    errors: list[str] = []
    for example in examples:
        validate_manifest(example, metadata, errors)

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"validated {len(examples)} device example(s) against {SCHEMA_PATH.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
