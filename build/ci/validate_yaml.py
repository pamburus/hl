#!/usr/bin/env python3
import os, sys, re, json
from pathlib import Path

import jsonschema
from jsonschema import FormatChecker
from ruamel.yaml import YAML
from ruamel.yaml.nodes import MappingNode, SequenceNode

from rich.console import Console
from rich.text import Text

# ---------- Config ----------
DEFAULT_TO_202012 = True  # set False to default to Draft-07 when $schema is absent
ROOT = Path(os.getenv("GITHUB_WORKSPACE") or Path.cwd()).resolve()
console = Console(stderr=True)  # colors enabled when terminal supports it
# ----------------------------

def relpath(p: Path) -> str:
    p = p.resolve()
    try:
        return p.relative_to(ROOT).as_posix()
    except ValueError:
        return p.as_posix()

def load_json(p: Path):
    with p.open("r", encoding="utf-8") as f:
        return json.load(f)

def load_yaml_data(text: str):
    y = YAML(typ="safe")
    return y.load(text)

def compose_yaml(text: str):
    y = YAML()
    return y.compose(text)

def choose_validator(schema: dict):
    meta = (schema.get("$schema") or "").lower()
    if "2020-12" in meta or "draft/2020-12" in meta:
        return jsonschema.Draft202012Validator, "Draft 2020-12"
    if "2019-09" in meta or "draft/2019-09" in meta:
        return jsonschema.Draft7Validator, "Draft 2019-09 (via Draft-07)"
    if "draft-07" in meta or "draft/7" in meta:
        return jsonschema.Draft7Validator, "Draft-07"
    if "draft-04" in meta or "draft4" in meta:
        return jsonschema.Draft4Validator, "Draft-04"
    return (
        (jsonschema.Draft202012Validator, "Draft 2020-12 (default)")
        if DEFAULT_TO_202012 else
        (jsonschema.Draft7Validator, "Draft-07 (default)")
    )

def pointer(parts):
    return "/" + "/".join(map(str, parts))

def build_lut_from_nodes(node, base_path=(), lut=None):
    """Map JSON-pointer tuple -> (line, col), 1-based, from ruamel node tree."""
    if lut is None:
        lut = {}
    if isinstance(node, MappingNode):
        for k_node, v_node in node.value:
            key = k_node.value
            path = base_path + (key,)
            lut[path] = (k_node.start_mark.line + 1, k_node.start_mark.column + 1)
            build_lut_from_nodes(v_node, path, lut)
    elif isinstance(node, SequenceNode):
        for idx, item_node in enumerate(node.value):
            path = base_path + (idx,)
            lut[path] = (item_node.start_mark.line + 1, item_node.start_mark.column + 1)
            build_lut_from_nodes(item_node, path, lut)
    return lut

_ADDP_RE = re.compile(r"Additional properties are not allowed \((.+?) were unexpected\)")
_PROP_RE = re.compile(r"'([^']+)'")

def extract_additional_props(msg: str):
    m = _ADDP_RE.search(msg)
    return _PROP_RE.findall(m.group(1)) if m else []

def code_frame(lines, line_no, col_no, width=120):
    if not (1 <= line_no <= len(lines)):
        return None
    src = lines[line_no - 1]
    if len(src) > width:
        start = max(0, col_no - width // 2)
        src = src[start:start + width]
        caret_col = min(col_no, width)
    else:
        caret_col = col_no
    caret = " " * max(0, caret_col - 1) + "^"
    return src, caret

def print_error(file_path_str, path_tuple, line_col, message, lines):
    loc = pointer(path_tuple) or "/"
    prefix = "  "
    header = (
        f"{prefix} {file_path_str}:{line_col[0]}:{line_col[1]} at {loc}: {message}"
        if line_col else f"{prefix} {file_path_str} at {loc}: {message}"
    )
    console.print(Text(header, style="bold red"))
    if line_col:
        cf = code_frame(lines, line_col[0], line_col[1])
        if cf:
            src, caret = cf
            console.print(Text(src))
            console.print(Text(caret, style="green"))

def validate_one(schema_path: Path, data_path: Path) -> bool:
    text = data_path.read_text(encoding="utf-8", errors="replace")
    data = load_yaml_data(text)
    root_node = compose_yaml(text)
    lines = text.splitlines()

    lut = build_lut_from_nodes(root_node)

    schema = load_json(schema_path)
    if "$id" not in schema:
        schema = dict(schema)
        schema["$id"] = schema_path.as_uri()

    Validator, draft_name = choose_validator(schema)
    Validator.check_schema(schema)
    v = Validator(schema, format_checker=FormatChecker())

    errors = sorted(v.iter_errors(data), key=lambda e: (list(e.absolute_path), e.message))
    if errors:
        console.print(Text(f"🔴 {relpath(data_path)} (using {draft_name})", style="bold red"))
        for e in errors:
            path_tuple = tuple(e.absolute_path)
            line_col = lut.get(path_tuple)

            if "Additional properties are not allowed" in e.message:
                for p in extract_additional_props(e.message):
                    p_path = path_tuple + (p,)
                    p_line_col = lut.get(p_path) or line_col
                    print_error(relpath(data_path), p_path, p_line_col, "unexpected property", lines)
                continue

            print_error(relpath(data_path), path_tuple, line_col, e.message, lines)
        return False

    console.print(Text(f"✅ {relpath(data_path)}", style="green"))
    return True

def main():
    if len(sys.argv) < 2:
        print("Usage: validate_yaml.py <schema.json> [data.yaml...]", file=sys.stderr)
        sys.exit(2)

    schema_path = Path(sys.argv[1]).resolve()
    files = [Path(p).resolve() for p in sys.argv[2:]]
    if not files:
        print(f"No files to validate against {relpath(schema_path)}")
        sys.exit(0)

    any_fail = False
    for f in files:
        if not validate_one(schema_path, f):
            any_fail = True

    sys.exit(1 if any_fail else 0)

if __name__ == "__main__":
    main()
