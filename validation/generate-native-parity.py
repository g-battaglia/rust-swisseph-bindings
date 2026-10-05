#!/usr/bin/env python3
"""Regenerate declaration-only parity inventories, never astronomical code.

This developer convenience is not used by Cargo or by the test runner. It reads
owned Rust public names and upstream public macro declarations to generate the
explicit operation/constant inventory. C callers still include swephexp.h and
are checked by the C compiler. No native implementation or fixture is read.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / "tests/native_parity"
DEST.mkdir(parents=True, exist_ok=True)

LAYOUTS = {}

def assign(layout, names):
    """Assign a public result projector to explicitly named operations."""
    for name in names.split():
        assert name not in LAYOUTS, name
        LAYOUTS[name] = layout

assign("Unit", "set_ephe_path set_jpl_file set_topo set_sid_mode set_tid_acc set_delta_t_userdef set_lapse_rate close require_source_flags")
assign("Text", "version library_path get_planet_name get_ayanamsa_name house_name cs2timestr cs2lonlatstr cs2degstr")
assign("PureScalar", "julday degnorm radnorm difdegn difdeg2n difrad2n deg_midp rad_midp")
assign("Scalar", "get_tid_acc get_ayanamsa get_ayanamsa_ut deltat sidtime sidtime0 refrac date_conversion")
assign("Integer", "day_of_week d2l csnorm difcsn difcs2n csroundsec")
assign("Position", "calc calc_ut calc_pctr")
assign("Calendar", "revjul")
assign("Utc", "jdet_to_utc jdut1_to_utc utc_time_zone")
assign("JulDays", "utc_to_jd")
assign("ScalarDiagnostic", "deltat_ex time_equ lmt_to_lat lat_to_lmt house_pos gauquelin_sector")
assign("Ayanamsha", "get_ayanamsa_ex get_ayanamsa_ex_ut")
assign("Houses", "houses houses_ex houses_armc")
assign("HouseSpeeds", "houses_ex2 houses_armc_ex2")
assign("Sectors", "houses_gauquelin")
assign("Star", "fixstar fixstar_ut fixstar2 fixstar2_ut")
assign("Magnitude", "fixstar_mag fixstar2_mag")
assign("Nodes", "nod_aps nod_aps_ut")
assign("Rise", "rise_trans rise_trans_true_hor")
assign("Vector", "cotrans cotrans_sp azalt azalt_rev refrac_extended")
assign("Phenomena", "pheno pheno_ut")
assign("Orbital", "get_orbital_elements")
assign("Distances", "orbit_max_min_true_distance")
assign("Crossing", "solcross solcross_ut mooncross mooncross_ut helio_cross helio_cross_ut")
assign("NodeCrossing", "mooncross_node mooncross_node_ut")
assign("GlobalEvent", "sol_eclipse_when_glob lun_eclipse_when lun_occult_when_glob lun_occult_when_glob_with_options")
assign("Geometry", "sol_eclipse_where lun_occult_where")
assign("Circumstances", "sol_eclipse_how lun_eclipse_how")
assign("LocalSolar", "sol_eclipse_when_loc")
assign("LocalEvent", "lun_eclipse_when_loc lun_occult_when_loc lun_occult_when_loc_with_options")
assign("Heliacal", "heliacal_ut")
assign("HeliacalPhenomena", "heliacal_pheno_ut")
assign("Visibility", "vis_limit_mag")
assign("Split", "split_deg")
assign("File", "get_current_file_data")

sources = {}
for path in sorted((ROOT / "src").glob("*.rs")):
    if path.name == "ffi.rs":
        continue
    for name in re.findall(r"^pub fn (\w+)\(", path.read_text(), re.M):
        sources[name] = path.stem
assert set(sources) == set(LAYOUTS), (set(sources) - set(LAYOUTS), set(LAYOUTS) - set(sources))
names = sorted(sources)
assert len(names) == 105
native = set(re.findall(r"pub\(crate\) fn (swe_\w+)", (ROOT / "src/ffi.rs").read_text()))
assert len(native) == 101

aliases = {"library_path": "swe_get_library_path", "houses_gauquelin": "swe_houses_ex2", "lun_occult_when_glob_with_options": "swe_lun_occult_when_glob", "lun_occult_when_loc_with_options": "swe_lun_occult_when_loc", "require_source_flags": ""}
symbols = {name: aliases.get(name, "swe_" + name) for name in names}
assert set(symbols.values()) - {""} == native

def variant(name):
    """Use an unambiguous Rust/C enum spelling for a canonical operation."""
    return "".join(word.capitalize() for word in name.split("_"))

text = "//! Generated operation accounting; see validation/generate-native-parity.py.\n\n"
text += "/// Every public free operation, with stable alphabetical wire identifiers.\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n#[repr(u32)]\npub enum Op {\n"
for i, name in enumerate(names):
    text += f"    /// Public `{name}`; native `{symbols[name] or 'Rust-only helper'}`.\n    {variant(name)} = {i},\n"
text += "}\n\n/// Independent C-buffer to public-result projection.\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum Layout {\n"
for layout in sorted(set(LAYOUTS.values())):
    text += f"    /// `{layout}` field ordering is defined in the independent comparator.\n    {layout},\n"
text += "}\n\n/// Declaration inventory entry, not a claim of executed coverage.\npub struct Operation {\n    /// Public free function identifier.\n    pub op: Op,\n    /// Canonical public name.\n    pub name: &'static str,\n    /// C symbol; empty only for the Rust-only source policy.\n    pub native: &'static str,\n    /// Owned Rust family containing the public entry point.\n    pub family: &'static str,\n    /// Result projection, including native padding.\n    pub layout: Layout,\n}\n\n/// Complete mapped public surface.\npub const OPERATIONS: &[Operation] = &[\n"
for name in names:
    text += f'    Operation {{ op: Op::{variant(name)}, name: "{name}", native: "{symbols[name]}", family: "{sources[name]}", layout: Layout::{LAYOUTS[name]} }},\n'
text += "];\n\nimpl Op {\n    /// Reject unknown wire identifiers instead of guessing a declaration.\n    pub fn from_wire(id: u32) -> Option<Self> { OPERATIONS.get(id as usize).map(|entry| entry.op) }\n    /// Retrieve the frozen declaration and result projection.\n    pub fn entry(self) -> &'static Operation { &OPERATIONS[self as usize] }\n}\n"
(DEST / "registry.rs").write_text(text)
header = "/* Generated public-operation wire identifiers. No native ABI redeclarations. */\n#ifndef PARITY_OPERATIONS_H\n#define PARITY_OPERATIONS_H\nenum parity_operation {\n"
for i, name in enumerate(names):
    header += f"    OP_{name.upper()} = {i},\n"
header += "};\n#endif\n"
(DEST / "operations.h").write_text(header)

# Resolve names to macros, leaving result-status aliases explicitly accounted for.
public = (ROOT / "swisseph/swephexp.h").read_text()
macros = set(re.findall(r"^#\s*define\s+(\w+)", public, re.M))
constants = re.findall(r'^pub const (\w+): (\w+) =\s+([^;]+);', (ROOT / "src/constants.rs").read_text(), re.M)
# The &str spelling needs a separate declaration pattern.
constants += re.findall(r'^pub const (\w+): (&str) = ([^;]+);', (ROOT / "src/constants.rs").read_text(), re.M)
assert len(constants) == 242, len(constants)
custom = {"HELFLAG_PHOTOPIC": "SE_PHOTOPIC_FLAG", "HELFLAG_SCOTOPIC": "SE_SCOTOPIC_FLAG", "HELFLAG_MIXED": "SE_MIXEDOPIC_FLAG", "HELFLAG_BELOW_HORIZON": "(-2)"}
ctypes = {"i32": [], "f64": [], "&str": []}
for name, typ, value in constants:
    candidates = [name, "SE_" + name, "SEFLG_" + name.removeprefix("FLG_")]
    macro = custom.get(name) or next((c for c in candidates if c in macros), None)
    assert macro, name
    ctypes[typ].append((name, macro))
cr = "//! Compiled-header constant checks; result-status aliases are explicit.\nuse super::protocol::Record;\nuse swisseph_bindings::*;\n\n/// Produce Rust constant values without parsing their numeric source literals.\npub fn rust_constants() -> Record {\n    Record { op: 1001, ints: vec![\n"
for name, macro in ctypes["i32"]:
    cr += f"        {name}, // C: {macro}\n"
cr += "    ], floats: vec![\n"
for name, macro in ctypes["f64"]:
    cr += f"        {name}, // C: {macro}\n"
cr += "    ], texts: vec![\n"
for name, macro in ctypes["&str"]:
    cr += f"        {name}.as_bytes().to_vec(), // C: {macro}\n"
cr += "    ], ..Record::default() }\n}\n"
(DEST / "constants.rs").write_text(cr)
cc = "/* Generated expressions are evaluated by the actual pinned public header. */\n"
for name, macro in ctypes["i32"]:
    cc += f"add_int(&result, {macro}); /* {name} */\n"
for name, macro in ctypes["f64"]:
    cc += f"add_float(&result, {macro}); /* {name} */\n"
for name, macro in ctypes["&str"]:
    cc += f"add_text(&result, {macro}, sizeof({macro}) - 1); /* {name} */\n"
(DEST / "constants.inc").write_text(cc)

ACCEPTED = {
    # Each row: executed evidence for branch acceptance, not mere declaration.
    "version": "smoke/data-pilot/full/sweep; 256-byte own-executable ABI text",
    "library_path": "smoke/data-pilot/full/sweep; per-worker path + long/multibyte boundary (Session state slice)",
    "get_planet_name": "classical/fictitious/asteroid-alias/missing-id native verdicts",
    "get_ayanamsa_name": "47 predefined + USER absent-name Native error",
    "house_name": "admitted letters incl. lowercase/unknown native fallback",
    "julday": "both calendars; BCE/year-zero/reform/leap/midnight/fractional",
    "revjul": "both calendars; negative/J2000/fractional JD",
    "utc_to_jd": "leap instants incl. 2016-12-31 60.5s; invalid non-leap 60.x",
    "jdet_to_utc": "representable final-UT1 regression under -1e12 override",
    "jdut1_to_utc": "representable final-UT1 regression under -1e12 override",
    "utc_time_zone": "fractional ±zones with date/year rollover",
    "date_conversion": "normalization vs native-error routes",
    "day_of_week": "J2000 band + historical epochs",
    "deltat": "historical/modern; manual/automatic tide; cleared override",
    "deltat_ex": "Swiss/Moshier/JPL sources; fallback diagnostics",
    "time_equ": "scalar + complete diagnostic vs native",
    "lmt_to_lat": "forward route vs native",
    "lat_to_lmt": "reverse route vs native",
    "sidtime": "range + explicit-frame agreement",
    "sidtime0": "agreement with sidtime path",
    "calc": "32k matrix v2 + 8k sweep; all bodies/flags/frames; ECL_NUT pctr gap documented",
    "calc_ut": "32k matrix v2 + 8k sweep; Swiss/Moshier/JPL(default/JPL-file) sources",
    "calc_pctr": "Swiss/JPL planet-centric success; nutation-target history gap excluded, no-speed neighbour kept",
    "set_ephe_path": "complete/empty/planet-only/moon-only; NUL/overlong/multibyte rejections",
    "set_jpl_file": "missing-file fallback; real de440.eph success; NUL/overlong rejections",
    "set_topo": "finite observers incl. 2467 m; NaN/height-boundary rejections",
    "set_sid_mode": "47 modes + USER ET/UT epochs/offsets; NaN rejections",
    "set_tid_acc": "explicit/automatic tides; lazy-init first-write scripts",
    "get_tid_acc": "explicit/automatic reads incl. DE441 -25.936 note",
    "set_delta_t_userdef": "pin/clear/restore incl. USER-UT preflight",
    "set_lapse_rate": "0/0.0065/0.01 sensitivity vs C rise geometry; non-finite rejections",
    "get_ayanamsa": "47 modes + USER paths vs native",
    "get_ayanamsa_ut": "47 modes + USER paths vs native",
    "get_ayanamsa_ex": "explicit-source ayanamsha vs native",
    "get_ayanamsa_ex_ut": "explicit-source UT ayanamsha vs native",
    "close": "explicit reset; post-close reuse compares",
    "get_current_file_data": "5 slots pre/post success/failure/reset/switch; Swiss+JPL DE numbers",
    "require_source_flags": "strict Swiss/Moshier/JPL unions on C-checked positions",
    "houses": "all ASCII letters; hemispheres/equator/polar; seams incl. ±0.0",
    "houses_ex": "same + sidereal + Sunshine declination input",
    "houses_ex2": "speeds + ascendant/MC getters; polar Native errors",
    "houses_armc": "ARMC systems vs native",
    "houses_armc_ex2": "ARMC speeds incl. Sunshine slot rewrite",
    "houses_gauquelin": "36 sectors + 8 angles + all speeds",
    "house_pos": "systems + polar/error routes",
    "gauquelin_sector": "body + star paths with catalog semantics",
    "fixstar": "traditional/nomenclature/number/wildcard; UT/TT/flags",
    "fixstar_ut": "same via UT entry point",
    "fixstar2": "extended positions; legacy preflight when catalog missing",
    "fixstar2_ut": "extended UT; legacy preflight when catalog missing",
    "fixstar_mag": "magnitudes incl. rewritten names",
    "fixstar2_mag": "extended magnitudes; legacy preflight routing",
    "nod_aps": "four 6-slot vectors, focal-point option",
    "nod_aps_ut": "same via UT entry point",
    "rise_trans": "rise/set/transits both hemispheres; Circumpolar absence; lapse/atmosphere sensitivity",
    "rise_trans_true_hor": "true-horizon route vs native",
    "azalt": "3 forward outputs, both frames",
    "azalt_rev": "2 reverse outputs, both frames",
    "refrac": "non-finite contract; finite altitude/details",
    "refrac_extended": "5-slot incl. dip + valid zeros",
    "pheno": "all 20 slots + 5 getters",
    "pheno_ut": "same via UT entry point",
    "get_orbital_elements": "all 50 slots incl. reserved tail + 17 getters",
    "orbit_max_min_true_distance": "all 3 distance outputs",
    "solcross": "ET longitude seam + forward/backward + sentinels",
    "solcross_ut": "same via UT entry point",
    "mooncross": "ET routes + sentinels",
    "mooncross_ut": "same via UT entry point",
    "mooncross_node": "node time + both coordinates",
    "mooncross_node_ut": "same via UT entry point",
    "helio_cross": "heliocentric directions vs native",
    "helio_cross_ut": "same via UT entry point",
    "sol_eclipse_when_glob": "global geometry incl. type-zero absence",
    "sol_eclipse_where": "geography + getters",
    "sol_eclipse_how": "20 attributes + circumstances",
    "sol_eclipse_when_loc": "local 27-slot incl. contacts",
    "lun_eclipse_when": "global lunar incl. backward search",
    "lun_eclipse_how": "lunar circumstances",
    "lun_eclipse_when_loc": "local 30-slot",
    "lun_occult_when_glob": "classic spelling vs search-control int",
    "lun_occult_when_glob_with_options": "options spelling vs search-control int",
    "lun_occult_when_loc": "local occultation",
    "lun_occult_when_loc_with_options": "options local occultation",
    "lun_occult_where": "occultation geometry",
    "heliacal_ut": "10-slot searches incl. failures",
    "heliacal_pheno_ut": "30-slot phenomena + 28 getters",
    "vis_limit_mag": "8 slots; -2 BelowHorizon with all slots",
    "cotrans": "rotations incl. seams/antipodes/signed zero",
    "cotrans_sp": "all 6 speed components",
    "degnorm": "seams + non-finite propagation",
    "radnorm": "seams + non-finite propagation",
    "difdegn": "pairs incl. antipodes",
    "difdeg2n": "pairs incl. antipodes",
    "difrad2n": "pairs incl. antipodes",
    "deg_midp": "midpoints",
    "rad_midp": "midpoints",
    "d2l": "ties + i32 endpoints",
    "csnorm": "centisecond normalization",
    "difcsn": "centisecond subtraction",
    "difcs2n": "centisecond subtraction variant",
    "csroundsec": "rounding ties/carries",
    "cs2timestr": "formatters + separators",
    "cs2lonlatstr": "direction bytes + lossy encoding",
    "cs2degstr": "degree formatter",
    "split_deg": "5 outputs under rounding/zodiacal/nakshatra/keep flags",
}
SESSION_ACCEPTED = {
    "new": "valid A/B construction; no install side effect",
    "ephe_path": "full/empty/default paths; PARTIAL masked inheritance",
    "default_ephe_path": "default reset individually + combined; Moshier probes",
    "jpl_file": "missing file; real de440.eph via SWISSEPH_JPL_DIR",
    "close_jpl_file": "close individually + combined",
    "topo": "Munich/Sydney + partial TOPO inheritance",
    "sid_mode": "Lahiri/USER-ET/USER-UT/reference/offset/pinned-DeltaT",
    "tid_acc": "explicit/automatic/first-write; inherited preflight restore",
    "delta_t_override": "explicit overrides; USER-UT preflight",
    "clear_delta_t_override": "clear individually + combined + preflight restore",
    "lapse_rate": "0/0.0065/0.01 sensitivity + atmosphere triple vs C",
    "build": "new/Default/cloned; 36 safe rejections preserve globals",
    "calc": "A/B/4+8 threads/NaN/history; A/B/A serial",
    "calc_ut": "A/B/4+8 threads/NaN/history; A/B/A serial",
    "houses": "A/B/threads/getters/NaN; polar Native errors",
    "houses_ex": "A/B/threads/getters/NaN",
    "get_ayanamsa_ut": "A/B/threads/NaN",
    "deltat": "A/B/threads/NaN",
    "rise_trans": "A/B/threads/NaN; Circumpolar absence",
    "get_current_file_data": "5 slots/threads/post-close/NaN/invalid; clone histories",
}
ledger = "# Native parity coverage manifest\n\nDeclaration accounting: 101 native symbols, 105 free functions, 242 constants.\nFull profile executes every row below; `full --require-jpl` adds the\nJPL-success block. No row is scientific acceptance. See `native-parity.md` for\nprojectors, safety exclusions and input identities.\n\n| Public operation | Native symbol | Family | Projector | Executed evidence | Coverage |\n| --- | --- | --- | --- | --- | --- |\n"
for name in names:
    prereq = "Owned manifests; file/source branches separately accounted"
    if sources[name] == "stars": prereq = "Explicit fixed-star catalog; missing extended catalog uses legacy preflight"
    if name == "require_source_flags": prereq = "Rust-only source policy; real native flags and diagnostic"
    state = "Full profile" if name in ACCEPTED else "Uncovered"
    evidence = ACCEPTED.get(name, prereq)
    ledger += f"| `{name}` | `{symbols[name] or 'none (Rust-only)'}` | {sources[name]} | {LAYOUTS[name]} | {evidence} | {state} |\n"
ledger += "\n## Constants\n\nAll 242 entries are retained in `tests/native_parity/constants.rs` and\n`constants.inc`; expressions are compiled through `swephexp.h`. Three\nHELFLAG vision aliases map to native result-marker macros. The below-horizon\nmarker `-2` is its documented native status contract with real BelowHorizon\noutcome coverage (all eight outputs). Path-length limits are binding-only\ncontracts with NUL/overlong/multibyte rejection plus exact boundary success.\nThe 243-field constants/marker gate passes in every profile.\n\n## Session / helper accounting\n\n| Public operation | Native reference | Executed evidence | Coverage |\n| --- | --- | --- | --- |\n"
for name in ["new", "ephe_path", "default_ephe_path", "jpl_file", "close_jpl_file", "topo", "sid_mode", "tid_acc", "delta_t_override", "clear_delta_t_override", "lapse_rate", "build"]:
    ledger += f"| `SessionBuilder::{name}` | Explicit public-ABI configuration script | {SESSION_ACCEPTED[name]} | Full profile |\n"
for name in ["calc", "calc_ut", "houses", "houses_ex", "get_ayanamsa_ut", "deltat", "rise_trans", "get_current_file_data"]:
    ledger += f"| `Session::{name}` | Matching C operation + owned configuration/history | {SESSION_ACCEPTED[name]} | Full profile |\n"
# Evidence is hand-maintained after execution. Regeneration must never reset
# verified rows or erase precise branch descriptions with template defaults.
ledger_path = ROOT / "validation/native-parity-coverage.md"
if not ledger_path.exists():
    ledger_path.write_text(ledger)
else:
    existing = ledger_path.read_text()
    assert all(f"| `{name}` |" in existing for name in names), "ledger declaration drift"

print("Generated declaration accounting: 101 native symbols / 105 free functions / 242 constants")
