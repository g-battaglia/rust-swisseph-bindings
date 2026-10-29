"""The bulk path migration must not change native call arguments or code."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("legacy_path_migration", Path(__file__).with_name("migrate-legacy-paths.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class LegacyPathMigrationTests(unittest.TestCase):
    def test_explicit_imports_keep_scalar_call_unchanged(self):
        source = "use swisseph_bindings::{MOON, nod_aps, library_path};\nnod_aps(jd, MOON, flags, method);\n"
        result = module.transform(source)
        self.assertIn("global::nod_aps", result)
        self.assertIn("native::library_path", result)
        self.assertTrue(result.endswith("nod_aps(jd, MOON, flags, method);\n"))

    def test_alias_and_existing_namespace(self):
        source = "use swisseph_bindings as swe;\nswe::calc_ut(jd, 1, 258); swe::global::calc(jd, 1, 258);"
        result = module.transform(source)
        self.assertIn("swe::global::calc_ut(jd, 1, 258)", result)
        self.assertNotIn("global::global", result)

    def test_glob_does_not_import_method_or_literal_names(self):
        source = 'use swisseph_bindings::*;\nsession.calc(jd, 1, 258);\nlet label = "calc_ut";\n'
        self.assertEqual(module.transform(source), source)

    def test_glob_imports_only_unqualified_legacy_entries(self):
        result = module.transform("use swisseph_bindings::*;\nlet p = calc_ut(jd, 1, 258);\n")
        self.assertIn("{global::calc_ut}", result)
        self.assertNotIn("global::calc,", result)

    def test_production_rewrite_is_doc_only(self):
        source = "/// use swisseph_bindings::calc_ut;\n/// See [`crate::calc_ut`].\npub fn helper() { crate::calc_ut(2451545.0, 1, 258); }\n"
        result = module.transform(source, source_docs=True)
        self.assertIn("/// use swisseph_bindings::global::calc_ut;", result)
        self.assertIn("[`crate::global::calc_ut`]", result)
        self.assertTrue(result.endswith("pub fn helper() { crate::calc_ut(2451545.0, 1, 258); }\n"))

    def test_namespace_rewrite_is_idempotent(self):
        source = "use swisseph_bindings::{calc_ut, Calendar};\n"
        once = module.transform(source)
        self.assertEqual(module.transform(once), once)


if __name__ == "__main__":
    unittest.main()
