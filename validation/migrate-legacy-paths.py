#!/usr/bin/env python3
"""Mechanically migrate owned legacy call sites to the frozen extension paths.

This maintenance codemod changes namespaces/imports only. It neither rewrites
function declarations nor arguments, assertions, numerical literals or engine
algorithms. The mapping is the reviewed 1.x public signature inventory. Run
without --apply to inspect the complete unified diff before applying it.
"""
from __future__ import annotations

import argparse
import difflib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
OPERATIONS = json.loads((ROOT / "validation/global-signatures-v1.json").read_text())["operations"]
DESTINATIONS = {row["name"]: row["legacy_destination"] for row in OPERATIONS}
DESTINATIONS.update({
    name: f"native::{name}" for name in (
        "CurrentFileData", "MAX_EPHE_PATH_LEN", "MAX_JPL_FILE_LEN",
        "HeliacalPhenomenaFull", "OccultationGeometryFull", "SolarEclipseGeometryFull",
        "VisibilityFull", "OccultSearchOptions",
    )
})
# Ignore prose/literals when deciding which formerly glob-imported names need
# an explicit import. Lifetimes are not character literals and remain intact.
NON_CODE = re.compile(r'//[^\n]*|/\*.*?\*/|r(#+)".*?"\1|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'', re.S)


def grouped_import(names: list[str]) -> str:
    """Render one explicit import without changing aliases or type spelling."""
    return "use swisseph_bindings::{" + ", ".join(
        DESTINATIONS.get(name.strip(), name.strip()) for name in names if name.strip()
    ) + "};"


def transform(text: str, *, source_docs: bool = False) -> str:
    """Return namespace-only edits for a Rust/Markdown text; never execute it."""
    if source_docs:
        # Production source changes are restricted to doc comments. Keeping
        # native operation bodies byte-identical is an intentional hard bound.
        lines = text.splitlines(keepends=True)
        out: list[str] = []
        index = 0
        while index < len(lines):
            line = lines[index]
            prefix = re.match(r"(\s*//[!/]) ?", line)
            if not prefix:
                out.append(line)
                index += 1
                continue
            block: list[str] = []
            marker = prefix[1]
            while index < len(lines):
                current = re.match(r"\s*//[!/] ?(.*?)(\n?)$", lines[index])
                if not current:
                    break
                block.append(current[1] + current[2])
                index += 1
            changed = transform("".join(block))
            # Doc comments also contain intra-doc links to old crate paths.
            for name, destination in DESTINATIONS.items():
                changed = re.sub(r"\bcrate::" + re.escape(name) + r"\b(?!::)", "crate::" + destination, changed)
            out.extend(marker + (" " if row.strip() else "") + row for row in changed.splitlines(keepends=True))
        return "".join(out)

    def explicit(match: re.Match[str]) -> str:
        body = match[1]
        # Existing nested imports already spell a deliberate namespace.
        if "{" in body or "}" in body:
            return match[0]
        return grouped_import(body.split(","))

    text = re.sub(r"use swisseph_bindings::\{([^{}]*)\};", explicit, text, flags=re.S)
    if "use swisseph_bindings::*;" in text:
        without_import = text.replace("use swisseph_bindings::*;", "")
        code = NON_CODE.sub("", without_import)
        already_imported: set[str] = set()
        for namespace, body in re.findall(r"use swisseph_bindings::(global|native)::\{([^}]+)\};", text, re.S):
            already_imported.update(re.findall(r"\b\w+\b", body))
        for body in re.findall(r"use swisseph_bindings::\{([^{}]+)\};", text, re.S):
            already_imported.update(re.findall(r"(?:global|native)::(\w+)", body))
        already_imported.update(re.findall(r"\bfn\s+(\w+)\s*\(", code))
        names = [name for name in DESTINATIONS if name not in already_imported and re.search(r"(?<![\w:.])" + re.escape(name) + r"\b", code)]
        if names:
            text = text.replace("use swisseph_bindings::*;", "use swisseph_bindings::*;\n" + grouped_import(names))
    aliases = {"swisseph_bindings"}
    aliases.update(re.findall(r"use swisseph_bindings as (\w+);", text))
    for alias in aliases:
        for name, destination in DESTINATIONS.items():
            text = re.sub(r"\b" + re.escape(alias) + "::" + re.escape(name) + r"\b(?!::)", alias + "::" + destination, text)
    return text


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="apply the reviewed namespace-only rewrite")
    args = parser.parse_args()
    candidates = sorted(ROOT.glob("tests/**/*.rs")) + sorted(ROOT.glob("examples/**/*.rs"))
    candidates += sorted(ROOT.glob("src/*.rs"))
    candidates += [ROOT / "README.md"]
    for path in candidates:
        if path.name in {"lib.rs", "global.rs", "native.rs", "common_api_contract.rs", "legacy_function_signatures.rs"}:
            continue
        before = path.read_text()
        after = transform(before, source_docs=path.parent == ROOT / "src")
        if before == after:
            continue
        print("".join(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
                                         fromfile=str(path.relative_to(ROOT)), tofile=str(path.relative_to(ROOT)))), end="")
        if args.apply:
            path.write_text(after)


if __name__ == "__main__":
    main()
