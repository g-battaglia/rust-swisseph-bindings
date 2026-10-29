# Configured sidereal state reuse

Configured sessions now retain an identical native sidereal mode rather than
calling `swe_set_sid_mode` before every computation. The pinned C setter clears
all apparent-position save areas even when its inputs are unchanged. Avoiding
that repeated configuration write leaves each requested calculation in C and
preserves the native engine's existing state reuse. No Rust result cache,
numerical correction, tolerance change or native engine modification is added.

The configuration key contains the exact mode, reference-epoch bits,
reference-offset bits and native thread identity. A public sidereal setter
always executes. Close clears the key. Every path and JPL setter invalidates
sidereal reuse because the native setter also clears astronomical models;
the existing changed-path replay can establish a fresh valid mode immediately.
Remembered mode fields remain available for that replay. This distinction is
necessary for the `SE_SIDBIT_PREC_ORIG` option: retaining the mode fields alone
does not retain its original precession model after a path or JPL write.

All metadata reads and writes follow the existing native-lock/metadata-lock
order. Poisoned metadata errors propagate, including from JPL configuration.
Raw C writes outside the binding's synchronization contract remain unsupported.
The native engine stays at `9083a12d59e98034fb2337061481ac8800c16e64`.

## Executed controls

`tests/session_sidereal_reuse.rs` builds an independent public-C baseline that
repeats the sidereal setter before every calculation. It compares all six exact
binary64 fields, returned source/flags and complete diagnostics against configured
Session operations. Four modes cover Lahiri, Fagan/Bradley with original
precession, True Sheoran and a user UT epoch. Two dates, six bodies and three
flag sets give 144 native baselines and 1,440 Session comparisons.

The ten lifecycle passes per mode cover initial/repeated calls, a conflicting
free mode, an equal forced path write, JPL selection, both close policies, an
equal forced sidereal write, two thread migrations and return to the parent
thread. Separate negative controls omit the path and JPL invalidations in turn;
each fails the exact C differential, and the restored implementation passes.
Native values and residuals remain in memory only.

The configured serial full suite passes 278 Rust tests and doctests, including
89 doctests, with no failures or ignored tests. Explicit contention tests keep
their internal worker threads. The unrestricted 1,347-case native smoke,
strict all-target Clippy, private Rustdoc with warnings denied and formatting
also pass. Timing is qualified separately on release artifacts.
