#!/usr/bin/env python3
"""Compare owned contract text and audit a compiler-resolved public surface.

This tool never imports an engine or reads native source/data. The inventory
input is rustdoc JSON generated locally with private AND hidden items retained;
otherwise a hidden public helper could evade the closed-surface check. Function
signatures and field types are checked by the Rust contract, not reconstructed
from rustdoc's version-specific type encoding here.

Commands:
  diff LOCAL_TEST PEER_TEST
  inventory RUSTDOC_JSON --scope root|api [--extension NAME ...]
  check RUSTDOC_JSON MANIFEST

The manifest specifies scope, permitted extension-container names, and every
allowed public path/kind. Inventory is explicit: this tool never updates an
allowlist automatically when checking it. Exit 1 means drift; malformed input
is an error, not a successful empty inventory.
"""
from __future__ import annotations

import argparse
import difflib
import json
from pathlib import Path
import sys

MARKER = b"// COMMON_API_CONTRACT_BEGIN\n"


def contract_body(path: Path) -> bytes:
    """Extract exactly one LF-terminated marker suffix; reject ambiguous files."""
    content = path.read_bytes()
    content.decode("utf-8", errors="strict")
    if b"\r" in content:
        raise ValueError(f"{path}: contract must use LF line endings")
    if content.count(MARKER) != 1:
        raise ValueError(f"{path}: expected exactly one contract marker")
    binding, body = content.split(MARKER)
    # Restrict the exception to one namespace binding, not arbitrary helper code.
    lines = [line.strip() for line in binding.decode().splitlines()
             if line.strip() and not line.lstrip().startswith("//")]
    if lines not in (["use swisseph_bindings as eph;"],
                     ["use libephemeris_core::api as eph;"]):
        raise ValueError(f"{path}: only the initial engine namespace binding may differ")
    if not body.strip():
        raise ValueError(f"{path}: empty contract body")
    return body


def compare_contracts(local: Path, peer: Path) -> bool:
    """Compare bytes first, rendering a human-readable diff only on mismatch."""
    left, right = contract_body(local), contract_body(peer)
    if left == right:
        print("contract suffixes are byte-identical")
        return True
    sys.stdout.writelines(difflib.unified_diff(
        left.decode().splitlines(keepends=True), right.decode().splitlines(keepends=True),
        fromfile=str(local), tofile=str(peer)))
    return False


class PublicSurface:
    """Traverse exports/re-exports and inherent members in one rustdoc graph.

    Private source modules can define public types re-exported by the primary
    module. Follow item IDs rather than source paths so their public fields and
    methods remain visible to the audit. Explicit trait implementations also
    enter the inventory: an unlisted Deref/extension trait must not smuggle
    plumbing into the effective API. Compiler-synthesized auto/blanket impls are
    excluded; required ownership/thread-safety traits are compiled separately.
    """

    def __init__(self, document: dict):
        if document.get("includes_private") is not True:
            raise ValueError("rustdoc JSON must include private items")
        self.index = document["index"]
        self.paths = document.get("paths", {})
        self.root = self.item(document["root"])
        if "module" not in self.root["inner"]:
            raise ValueError("rustdoc root is not a module")
        self.local_crate = self.root["crate_id"]
        self.exports: dict[str, str] = {}

    def item(self, identity):
        try:
            return self.index[str(identity)]
        except KeyError as error:
            raise ValueError(f"unresolved rustdoc item {identity!r}") from error

    @staticmethod
    def public(item: dict) -> bool:
        return item["visibility"] == "public"

    def record(self, path: str, kind: str) -> None:
        if path in self.exports and self.exports[path] != kind:
            raise ValueError(f"ambiguous public path {path}")
        self.exports[path] = kind

    def children(self, module: dict, prefix: str, ancestors: frozenset,
                 extensions: frozenset[str]) -> None:
        for identity in module["inner"]["module"]["items"]:
            item = self.item(identity)
            if not self.public(item):
                continue
            inner = item["inner"]
            if "use" in inner:
                use = inner["use"]
                if use["id"] is None:
                    raise ValueError(f"unresolved public import: {use['source']}")
                target = self.item(use["id"])
                if use["is_glob"]:
                    if "module" not in target["inner"]:
                        raise ValueError("public glob target must resolve to a module")
                    identity = str(target["id"])
                    if identity in ancestors:
                        raise ValueError("cyclic public glob traversal")
                    self.children(target, prefix, ancestors | {identity}, extensions)
                else:
                    self.visit(target, use["name"], prefix, ancestors, extensions)
            else:
                self.visit(item, item["name"], prefix, ancestors, extensions)

    def visit(self, item: dict, name: str, prefix: str, ancestors: frozenset,
              extensions: frozenset[str]) -> None:
        path = f"{prefix}::{name}" if prefix else name
        kind, value = next(iter(item["inner"].items()))
        self.record(path, kind)
        if not prefix and name in extensions:
            if kind != "module":
                raise ValueError(f"extension {name} is not a module")
            return
        identity = str(item["id"])
        if identity in ancestors:
            raise ValueError(f"cyclic primary module/type traversal: {path}")
        ancestors = ancestors | {identity}
        if kind == "module":
            self.children(item, path, ancestors, frozenset())
        elif kind == "struct":
            shape = value["kind"]
            if "plain" in shape:
                self.members(shape["plain"]["fields"], path, public_only=True)
            elif "tuple" in shape:
                self.tuple_fields(shape["tuple"], path, public_only=True)
            self.impls(value["impls"], path)
        elif kind == "enum":
            for variant_id in value["variants"]:
                variant = self.item(variant_id)
                variant_path = f"{path}::{variant['name']}"
                self.record(variant_path, "variant")
                shape = variant["inner"]["variant"]["kind"]
                if isinstance(shape, dict) and "struct" in shape:
                    self.members(shape["struct"]["fields"], variant_path, public_only=False)
                elif isinstance(shape, dict) and "tuple" in shape:
                    self.tuple_fields(shape["tuple"], variant_path, public_only=False)
            self.impls(value["impls"], path)
        elif kind == "type_alias":
            # An alias can expose the target's fields/methods. Do not silently
            # inventory only the alias name and miss an inherited public helper.
            target_type = value["type"]
            if "resolved_path" not in target_type:
                # Tuple/scalar aliases cannot define new inherent methods.
                # The alias itself remains in the closed export allowlist.
                return
            target = self.item(target_type["resolved_path"]["id"])
            if target["crate_id"] != self.local_crate:
                raise ValueError(f"external primary type alias is not supported: {path}")
            before = self.exports[path]
            del self.exports[path]
            self.visit(target, name, prefix, ancestors - {identity}, frozenset())
            self.exports[path] = before
        elif kind == "trait":
            self.members(value["items"], path, public_only=False)
        elif kind == "union":
            self.members(value["fields"], path, public_only=True)
            self.impls(value["impls"], path)

    def members(self, identities: list, prefix: str, public_only: bool) -> None:
        for identity in identities:
            item = self.item(identity)
            if not public_only or self.public(item):
                kind = next(iter(item["inner"]))
                # Rust permits a field and an associated function with the same
                # name. Use member-access syntax for fields to keep both entries.
                separator = "." if kind == "struct_field" else "::"
                self.record(f"{prefix}{separator}{item['name']}", kind)

    def tuple_fields(self, identities: list, prefix: str, public_only: bool) -> None:
        for position, identity in enumerate(identities):
            if identity is not None:
                field = self.item(identity)
                if not public_only or self.public(field):
                    self.record(f"{prefix}.{position}", "struct_field")

    def impls(self, identities: list, prefix: str) -> None:
        for identity in identities:
            implementation = self.item(identity)["inner"]["impl"]
            trait = implementation["trait"]
            if trait is None:
                self.members(implementation["items"], prefix, public_only=True)
            elif not implementation.get("is_synthetic", False) and implementation.get("blanket_impl") is None:
                summary = self.paths.get(str(trait["id"]))
                name = "::".join(summary["path"]) if summary else trait["path"]
                self.record(f"{prefix}#impl {name}", "trait_impl")

    def inventory(self, scope: str, extensions: list[str]) -> dict[str, str]:
        module = self.root
        if scope != "root":
            candidates = [self.item(identity)
                          for identity in module["inner"]["module"]["items"]]
            matches = [item for item in candidates if item["name"] == scope
                       and self.public(item) and "module" in item["inner"]]
            if len(matches) != 1:
                raise ValueError(f"expected one public scope module {scope!r}")
            module = matches[0]
        self.children(module, "", frozenset({str(module["id"])}), frozenset(extensions))
        for name in extensions:
            if self.exports.get(name) != "module":
                raise ValueError(f"declared extension container missing: {name}")
        return dict(sorted(self.exports.items()))


def check_inventory(actual: dict, expected: dict) -> bool:
    """Fail both unexpected exports and removals; never bless drift by rewriting."""
    lines = []
    for path in sorted(actual.keys() | expected.keys()):
        if actual.get(path) != expected.get(path):
            lines.append(f"{path}: expected {expected.get(path)!r}, found {actual.get(path)!r}")
    if lines:
        print("\n".join(lines))
        return False
    print(f"closed public inventory matches: {len(actual)} entries")
    return True


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    diff = commands.add_parser("diff")
    diff.add_argument("local", type=Path)
    diff.add_argument("peer", type=Path)
    inventory = commands.add_parser("inventory")
    inventory.add_argument("document", type=Path)
    inventory.add_argument("--scope", choices=["root", "api"], required=True)
    inventory.add_argument("--extension", action="append", default=[])
    check = commands.add_parser("check")
    check.add_argument("document", type=Path)
    check.add_argument("manifest", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "diff":
            return 0 if compare_contracts(args.local, args.peer) else 1
        document = json.loads(args.document.read_text())
        if args.command == "inventory":
            result = PublicSurface(document).inventory(args.scope, args.extension)
            print(json.dumps(result, indent=2, sort_keys=True))
            return 0
        manifest = json.loads(args.manifest.read_text())
        if manifest.get("pending"):
            raise ValueError("primary contract has pending mappings; partial coverage is not acceptance")
        actual = PublicSurface(document).inventory(manifest["scope"], manifest["extensions"])
        return 0 if check_inventory(actual, manifest["exports"]) else 1
    except (KeyError, TypeError, ValueError, OSError) as error:
        parser.exit(2, f"contract check error: {error}\n")


if __name__ == "__main__":
    sys.exit(main())
