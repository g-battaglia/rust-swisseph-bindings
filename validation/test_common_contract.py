"""Data-independent negative controls for the local primary API audit.

Run with `python3 -m unittest discover -s validation -p test_common_contract.py`.
Synthetic graphs model public Rust declarations only, never numerical vectors.
"""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "common_contract", Path(__file__).with_name("check-common-contract.py"))
CONTRACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CONTRACT)


def item(identity, name, inner, visibility="public"):
    return {"id": identity, "name": name, "inner": inner,
            "visibility": visibility, "crate_id": 0}


def graph():
    """A private source module re-exporting an opaque Session with one method."""
    return {"root": 0, "includes_private": True, "index": {
        "0": item(0, "engine", {"module": {"items": [1, 7]}}),
        "1": item(1, "Session", {"use": {"id": 2, "name": "Session",
                                            "is_glob": False, "source": "private::Session"}}),
        "2": item(2, "Session", {"struct": {
            "kind": {"plain": {"fields": [3]}}, "impls": [4]}}),
        "3": item(3, "secret", {"struct_field": {"primitive": "u8"}}, "default"),
        "4": item(4, None, {"impl": {"trait": None, "items": [5]}}, "default"),
        "5": item(5, "calc", {"function": {}}, "public"),
        "7": item(7, "native", {"module": {"items": [8]}}),
        "8": item(8, "backend_only", {"function": {}}),
    }}


class PublicInventoryTests(unittest.TestCase):
    def inventory(self, document):
        return CONTRACT.PublicSurface(document).inventory("root", ["native"])

    def test_reexported_type_methods_and_extension_boundary(self):
        self.assertEqual(self.inventory(graph()), {
            "Session": "struct", "Session::calc": "function", "native": "module"})

    def test_extra_root_export_is_rejected(self):
        document = graph()
        expected = self.inventory(document)
        document["index"]["0"]["inner"]["module"]["items"].append(9)
        document["index"]["9"] = item(9, "accidental", {"function": {}})
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertFalse(CONTRACT.check_inventory(self.inventory(document), expected))

    def test_hidden_extra_session_method_is_rejected(self):
        document = graph()
        expected = self.inventory(document)
        document["index"]["4"]["inner"]["impl"]["items"].append(9)
        document["index"]["9"] = item(9, "plumbing", {"function": {}})
        document["index"]["9"]["attrs"] = ["#[doc(hidden)]"]
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertFalse(CONTRACT.check_inventory(self.inventory(document), expected))

    def test_deref_trait_cannot_hide_an_extra_effective_api(self):
        document = graph()
        expected = self.inventory(document)
        document["index"]["2"]["inner"]["struct"]["impls"].append(9)
        document["index"]["9"] = item(9, None, {"impl": {
            "trait": {"id": 99, "path": "core::ops::Deref"}, "items": [],
            "is_synthetic": False, "blanket_impl": None}}, "default")
        actual = self.inventory(document)
        self.assertEqual(actual["Session#impl core::ops::Deref"], "trait_impl")
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertFalse(CONTRACT.check_inventory(actual, expected))

    def test_field_and_associated_method_can_share_a_name(self):
        document = graph()
        document["index"]["3"]["visibility"] = "public"
        document["index"]["3"]["name"] = "calc"
        surface = self.inventory(document)
        self.assertEqual(surface["Session.calc"], "struct_field")
        self.assertEqual(surface["Session::calc"], "function")

    def test_renamed_export_alias_does_not_hide_members(self):
        document = graph()
        document["index"]["1"]["inner"]["use"]["name"] = "Renamed"
        self.assertIn("Renamed::calc", self.inventory(document))

    def test_public_field_detected_but_private_storage_not_exposed(self):
        document = graph()
        document["index"]["3"]["visibility"] = "public"
        self.assertEqual(self.inventory(document)["Session.secret"], "struct_field")

    def test_private_items_required_and_unresolved_imports_rejected(self):
        document = graph()
        document["includes_private"] = False
        with self.assertRaises(ValueError):
            self.inventory(document)
        document = graph()
        document["index"]["1"]["inner"]["use"]["id"] = None
        with self.assertRaises(ValueError):
            self.inventory(document)

    def test_type_alias_follows_public_methods(self):
        document = graph()
        document["index"]["0"]["inner"]["module"]["items"].append(9)
        document["index"]["9"] = item(9, "Alias", {"type_alias": {
            "type": {"resolved_path": {"id": 2}}}})
        surface = self.inventory(document)
        self.assertEqual(surface["Alias"], "type_alias")
        self.assertEqual(surface["Alias::calc"], "function")

    def test_enums_include_payload_fields_and_variants(self):
        document = graph()
        document["index"]["0"]["inner"]["module"]["items"].append(9)
        document["index"]["9"] = item(9, "Outcome", {"enum": {"variants": [10], "impls": []}})
        document["index"]["10"] = item(10, "Found", {"variant": {"kind": {
            "struct": {"fields": [11]}}}}, "default")
        document["index"]["11"] = item(11, "time", {"struct_field": {}}, "default")
        self.assertIn("Outcome::Found.time", self.inventory(document))

    def test_extra_extension_name_or_nonmodule_rejected(self):
        document = graph()
        document["index"]["7"]["inner"] = {"function": {}}
        with self.assertRaises(ValueError):
            self.inventory(document)


    def test_pending_contract_is_not_accepted(self):
        with tempfile.TemporaryDirectory() as directory:
            doc = Path(directory) / "doc.json"
            manifest = Path(directory) / "contract.json"
            doc.write_text(json.dumps(graph()))
            manifest.write_text(json.dumps({"pending": ["Session::remaining_operation"]}))
            with contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as failure:
                    CONTRACT.main(["check", str(doc), str(manifest)])
            self.assertEqual(failure.exception.code, 2)


class ContractTextTests(unittest.TestCase):
    def test_only_binding_block_can_differ(self):
        with tempfile.TemporaryDirectory() as directory:
            local, peer = [Path(directory) / name for name in ("local.rs", "peer.rs")]
            body = CONTRACT.MARKER + b"fn contract() {}\n"
            local.write_bytes(b"use swisseph_bindings as eph;\n" + body)
            peer.write_bytes(b"use libephemeris_core::api as eph;\n" + body)
            self.assertEqual(CONTRACT.contract_body(local), CONTRACT.contract_body(peer))
            peer.write_bytes(b"use libephemeris_core::api as eph;\nfn disguise() {}\n" + body)
            with self.assertRaises(ValueError):
                CONTRACT.contract_body(peer)

    def test_missing_repeated_marker_or_crlf_is_an_error(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "test.rs"
            for content in (b"", CONTRACT.MARKER * 2,
                            b"use swisseph_bindings as eph;\r\n" + CONTRACT.MARKER):
                path.write_bytes(content)
                with self.assertRaises(ValueError):
                    CONTRACT.contract_body(path)


if __name__ == "__main__":
    unittest.main()
