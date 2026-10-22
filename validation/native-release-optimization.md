# Native release archive optimization

Release builds at optimization level 3 use ordinary `-O3`. On Darwin, the
archive additionally contains compiler bitcode (`-flto`) so the native linker
can optimize across the original C translation units. Other targets and
development builds keep ordinary object files. No source, data, floating-point
precision, fast-math option or CPU-specific instruction flag changes.

The native input manifest records the actual flags and compiler. The engine
remains `9083a12d59e98034fb2337061481ac8800c16e64`; the separately attributed
versioned split utility remains an additional explicit translation unit.

On native macOS arm64, independent original-engine O2 and O3/LTO libraries
were compared in memory with identical data and configuration. All 90 position,
45 star and 18 house requests matched bit-for-bit, including output arrays,
returned status/flags, names and diagnostic text. The date inputs include JD
1700000.5, 2451545.0 and 2460310.5. No numerical vectors or residuals are saved.

The focused release library/input-domain/time/core/legacy-split/house/star
tests pass (54 tests). The existing filtered direct-C angle, position, house
and star parity controls also pass. Formatting and strict all-target Clippy
pass. These focused results do not close the separately recorded unrestricted
session reset/history smoke finding in `versioned-split.md`, or establish
independent scientific certification.
