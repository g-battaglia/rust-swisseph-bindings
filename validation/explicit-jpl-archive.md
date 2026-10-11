# Explicit DE440 archive verification — 2026-10-11

The differential runner now accepts an explicitly selected full-file archive
through SWISSEPH_JPL_FILE. It retains the historical directory/basename/MD5
recipe separately. No file is copied, renamed, downloaded or numerically fitted.
The actual selected file is fingerprinted before and after comparison.

Selected identity: DE440/LE440, 102272352 bytes, JD2287184.5..2688976.5,
SHA25629915576d0a6555766b99485ac3056ee415e86df4fce282611c31afb329ad062.
Native C remains 9083a12d59e98034fb2337061481ac8800c16e64, version2.10.03.
The selected Swiss/catalog inputs match the previously recorded identities.

## Executed checks

- Historical directory recipe with only the differently named archive: exit1,
  missing historical basename; no new archive silently substituted.
- Explicit SPK archive with a different identity: exit1, SHA256 mismatch before
  native comparison.
- Data-pilot sources with the selected DE440 and --require-jpl: exit0,
  134cases, 12public operations, 11native symbols, 3020comparison/rejection fields
  plus243constant/status fields;655successes,83native errors. Genuine JPL flags
  and source policies compare exactly against the independent C caller.
- Current local smoke: exit0,1297cases,13811comparison/rejection fields plus243
  constant/status fields;1268successes,57native errors,15absences,45rejections.
- Strict all-target Clippy and formatting pass.

Outputs/protocol frames stayed in memory. Filtered source execution is not a
full matrix, scientific certification or extracted-package verification.
