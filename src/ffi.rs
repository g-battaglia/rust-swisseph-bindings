//! Private boundary for native ABI declarations and unsafe calls.
//!
//! Raw symbols stay private: the public API exposes only safe wrappers that
//! validate their preconditions, hold the process-wide native lock from
//! [`crate::state`], and copy every result into owned Rust storage before
//! any native lifetime can end.
//!
//! Declaration provenance (pinned `swisseph/swephexp.h` + `sweodef.h`):
//!
//! - Calling convention is plain C (`CALL_CONV` is empty on Unix).
//! - `int32` is `int` on every non-16-bit target, hence Rust `i32`.
//!   A build-time C probe (`_Static_assert(sizeof(int32) == 4)` in
//!   `build.rs`) rejects any platform where this does not hold instead of
//!   guessing widths.
//! - `double` is Rust `f64` (probed as 8 bytes), `char` is [`c_char`].
//! - Diagnostics, planet names and version use the 256-byte `AS_MAXCH`
//!   convention. Library path output needs 257 bytes; fixed-star names
//!   need 512 bytes. Every read is bounded by the actual buffer capacity and a
//!   missing terminator degrades to a lossy read, never to an over-read.
//! - `swe_set_ephe_path` copies its argument into native storage, so a
//!   temporary `CString` is safe to pass. Returned `char *` values alias
//!   either our own buffer or native static storage; both are copied into
//!   owned `String`s before the lock is released and no pointer escapes.

use std::ffi::{CStr, c_char, c_double, c_int};

/// Capacity of native diagnostic (`serr`) buffers.
///
/// The native API documents 256-byte `serr` buffers (`AS_MAXCH` in the
/// pinned `sweodef.h`).
pub(crate) const SERR_LEN: usize = 256;

/// Capacity of the version and planet-name buffers.
///
/// Both follow the same 256-byte convention; planet names written by the
/// native library are short literals or catalog names well below this.
pub(crate) const TEXT_BUF_LEN: usize = 256;

/// Capacity of `swe_get_library_path` output at the pinned revision.
/// The non-astronomical path helper copies up to `AS_MAXCH` bytes and
/// writes the terminator at that index; it therefore needs 257 bytes.
pub(crate) const LIBRARY_PATH_LEN: usize = 257;

/// Native output width of one position vector: longitude, latitude,
/// distance plus the three matching daily rates.
pub(crate) const POSITION_LEN: usize = 6;

/// Capacity of the fixed-star name buffer.
///
/// The pinned `swephexp.h` documents `SE_MAX_STNAME` (256) as the maximum
/// fixstar name size and requires the `star` parameter of `swe_fixstar` to
/// allow twice that space: the engine overwrites the input search string
/// with the resolved full name (`"Name,Nomenclature"`), which is longer
/// than the query. Every star buffer therefore owns 512 bytes.
pub(crate) const STAR_BUF_LEN: usize = 512;

/// Native `attr` width for phenomena results: phase angle, illuminated
/// fraction, elongation, apparent diameter and magnitude in slots 0-4
/// with the remaining slots zeroed by the engine (established by probing
/// the pinned build: slots 5-19 read back as 0.0, never uninitialized).
pub(crate) const PHENO_LEN: usize = 20;

/// Compatibility width of the orbital-element vector.
///
/// The pinned engine writes the 17 osculating-element slots 0-16 and
/// leaves the rest untouched (established by probing Mars, Moon and
/// Jupiter: slots 17-49 kept their sentinel values). The safe wrapper
/// passes a zero-initialized 50-slot buffer and exposes all of it, so the
/// reserved tail reads as 0.0 and no uninitialized memory is exposed.
pub(crate) const ORBEL_LEN: usize = 50;

/// Native `dret` width for the extended refraction details: true
/// altitude, apparent altitude, refraction amount and horizon dip
/// (established by probing the pinned build against the compatibility
/// layout).
pub(crate) const REFRACTION_DETAILS_LEN: usize = 4;

/// Native `xin`/`xaz` width for the horizontal-coordinate conversions.
///
/// `swe_azalt` reads the input pair and writes all three output slots
/// (azimuth, true altitude, apparent altitude); `swe_azalt_rev` reads the
/// input pair and writes exactly two output slots (established by probing
/// the pinned build: a sentinel third output slot is never touched).
/// The third input slot is provably ignored by the engine (varied across
/// 0/1/100 with bitwise-identical output under both direction flags), so
/// the safe wrappers pass an initialized 0.0 there.
pub(crate) const HORIZONTAL_LEN: usize = 3;

/// Native output width of the reverse horizontal conversion: the
/// celestial pair only (right ascension/declination or ecliptic
/// longitude/latitude).
pub(crate) const CELESTIAL_LEN: usize = 2;

/// Native `tret` width for the global eclipse searches: maximum plus the
/// successive contact/center-line times (ten slots; entries that do not
/// apply to the found type read back as 0.0).
pub(crate) const ECLIPSE_TRET_LEN: usize = 10;

/// Native `attr` width for the eclipse circumstance calls.
///
/// The pinned `swecl.c` API comments require callers to declare at least
/// 20 slots (`declare as attr[20] at least !`); the engine fills slots
/// 0-10 (magnitude, ratios, obscuration, core width, azimuth, altitudes,
/// separation, NASA magnitude, saros series/member). The safe wrappers
/// pass a zero-initialized 20-slot buffer and expose all of it, so the
/// reserved tail reads as 0.0 and no uninitialized memory is exposed.
pub(crate) const ECLIPSE_ATTR_LEN: usize = 20;

/// Native `geopos` width written by the eclipse `where` calls: eastern
/// longitude and northern latitude in degrees of the greatest eclipse.
pub(crate) const ECLIPSE_GEOPOS_LEN: usize = 2;

/// Native `tret` width for the local solar-eclipse and occultation
/// searches: maximum, first/second/third/fourth contact, sunrise, sunset
/// (seven slots; entries that do not apply read back as 0.0).
///
/// Note: the occultation search zeroes ten slots on entry (see
/// [`ECLIPSE_TRET_LEN`]); its safe wrapper passes a ten-slot buffer and
/// exposes all of it, with slots 7-9 documented as reserved.
pub(crate) const LOCAL_SOLAR_TRET_LEN: usize = 7;

/// Native `dret` width for `swe_heliacal_ut`: beginning, optimum and end
/// of visibility (slots 0-2) plus seven further slots.
///
/// Established by probing the pinned build with sentinel-filled buffers:
/// the visibility-limit planet path writes all ten slots (0.0 in 3-9),
/// the Moon path writes only slots 0-2, and the arcus-visionis
/// (`SE_HELFLAG_AVKIND`) path writes only slot 0 but is rejected by the
/// safe search wrapper because of a separate native internal-buffer defect.
/// The safe wrapper passes
/// a zero-initialized ten-slot buffer and exposes all of it, so the
/// unwritten slots read as 0.0 — matching the documented "0 if
/// `SE_HELFLAG_AV`" contract — and no uninitialized memory is exposed.
pub(crate) const HELIACAL_DRET_LEN: usize = 10;

/// Native `darr` width for `swe_heliacal_pheno_ut`: the 28 phenomenon
/// components 0-27 (object/Sun geometry, arcus visionis, extinction,
/// visibility window, lunar/Yallop details, magnitude, rise/set times)
/// plus two reserved slots.
///
/// Established by probing the pinned build (Venus, Moon and arcus-visionis
/// paths): slots 0-27 are engine-written while slots 28-29 keep their
/// sentinel values. The safe wrapper passes a zero-initialized 30-slot
/// buffer and exposes all of it, so the reserved tail reads as 0.0 and no
/// uninitialized memory is exposed.
pub(crate) const HELIACAL_PHENO_LEN: usize = 30;

/// Native `dret` width for `swe_vis_limit_mag`: the visual limiting
/// magnitude (slot 0), object/Sun geometry details (slots 1-4 and 7) and
/// two further engine details (slots 5-6).
///
/// Established by probing the pinned build with sentinel-filled buffers:
/// a visible instant writes all eight slots, while the below-horizon and
/// error paths write only slots 0-6 (slot 0 carries -100 below the
/// horizon, 0.0 on error). The safe wrapper passes a zero-initialized
/// eight-slot buffer and exposes all of it.
pub(crate) const VISLIM_DRET_LEN: usize = 8;

/// Native `dgeo` width for the heliacal calls: eastern longitude
/// (degrees), latitude (degrees), eye height (meters above sea level).
pub(crate) const HELIACAL_GEO_LEN: usize = 3;

/// Native `datm` width for the heliacal calls: atmospheric pressure
/// (mbar), temperature (°C), relative humidity (%), meteorological range
/// or total extinction coefficient. Zero entries select native defaults.
pub(crate) const HELIACAL_ATM_LEN: usize = 4;

/// Native `dobs` width for the heliacal calls: observer age (years),
/// Snellen visual-acuity factor, binocular flag, telescope magnification,
/// aperture (mm), transmission. Zero entries select native defaults; the
/// last four apply only with `SE_HELFLAG_OPTICAL_PARAMS`.
pub(crate) const HELIACAL_OBS_LEN: usize = 6;

/// Native output width of the formatted angle/time writers
/// (`swe_cs2timestr`, `swe_cs2lonlatstr`, `swe_cs2degstr`).
///
/// The longest probed output is the 9-byte degree string (UTF-8 degree
/// sign included); a 256-byte caller buffer therefore leaves a wide
/// margin and every read stays bounded by this capacity.
pub(crate) const FORMAT_BUF_LEN: usize = 256;

/// Native `cusps` width for the standard twelve-house systems.
///
/// Index 0 is reserved by the native layout; entries 1-12 hold houses 1-12.
/// The Gauquelin sector layout (`'G'`) writes 36 entries and is rejected by
/// the twelve-house wrappers; the separate Gauquelin wrapper uses 37 slots.
pub(crate) const CUSPS_NATIVE_LEN: usize = 13;

/// Native `cusps` width for the Gauquelin 36-sector layout.
///
/// Index 0 is reserved by the native layout; entries 1-36 hold sectors
/// 1-36. Speed buffers (`cusp_speed`) share this width.
pub(crate) const CUSPS_GAUQUELIN_LEN: usize = 37;

/// Native `ascmc` width: ASC, MC, ARMC, Vertex, Equatorial ASC, Koch and
/// Munkasey co-ascendants, polar ascendant, plus two reserved slots.
pub(crate) const ASCMC_NATIVE_LEN: usize = 10;

unsafe extern "C" {
    /// `char *swe_version(char *svers)` — write the library version into
    /// the caller buffer and return it. The buffer must hold at least
    /// [`TEXT_BUF_LEN`] bytes.
    pub(crate) fn swe_version(svers: *mut c_char) -> *mut c_char;

    /// `char *swe_get_library_path(char *spath)` — write the native
    /// executable/library pathname, possibly truncated to 256 bytes.
    /// The caller must provide [`LIBRARY_PATH_LEN`] bytes.
    pub(crate) fn swe_get_library_path(spath: *mut c_char) -> *mut c_char;

    /// `double swe_julday(int year, int month, int day, double hour,
    /// int gregflag)` — pure calendar arithmetic, no failure mode.
    pub(crate) fn swe_julday(
        year: c_int,
        month: c_int,
        day: c_int,
        hour: c_double,
        gregflag: c_int,
    ) -> c_double;

    /// `void swe_revjul(double jd, int gregflag, int *jyear, int *jmon,
    /// int *jday, double *jut)` — pure calendar arithmetic; all out
    /// pointers must be valid.
    pub(crate) fn swe_revjul(
        jd: c_double,
        gregflag: c_int,
        jyear: *mut c_int,
        jmon: *mut c_int,
        jday: *mut c_int,
        jut: *mut c_double,
    );

    /// `int32 swe_calc_ut(double tjd_ut, int32 ipl, int32 iflag,
    /// double *xx, char *serr)` — `xx` needs [`POSITION_LEN`] slots,
    /// `serr` needs [`SERR_LEN`] bytes. Returns the (possibly adjusted)
    /// flag set on success, a negative value on failure.
    pub(crate) fn swe_calc_ut(
        tjd_ut: c_double,
        ipl: i32,
        iflag: i32,
        xx: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_calc(double tjd, int ipl, int32 iflag, double *xx,
    /// char *serr)` — same contract as `swe_calc_ut` in Ephemeris Time.
    pub(crate) fn swe_calc(
        tjd_et: c_double,
        ipl: c_int,
        iflag: i32,
        xx: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_calc_pctr(double tjd, int32 ipl, int32 iplctr, int32 iflag,
    /// double *xxret, char *serr)` — planet-centric position of body `ipl`
    /// as seen from body `iplctr`, in Ephemeris Time only (there is no UT
    /// variant natively). `xxret` needs [`POSITION_LEN`] slots, `serr`
    /// needs [`SERR_LEN`] bytes; negative return means failure.
    pub(crate) fn swe_calc_pctr(
        tjd_et: c_double,
        ipl: i32,
        iplctr: i32,
        iflag: i32,
        xxret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `void swe_set_jpl_file(const char *fname)` — select the JPL file for
    /// `FLG_JPLEPH` calculations. The argument must be non-null (the native
    /// setter reads it with `strlen` first); it is copied into native
    /// storage, so a temporary `CString` is safe. Names of `AS_MAXCH`
    /// (256) bytes or more are silently truncated natively, hence rejected
    /// by the safe wrapper. The call closes cached file data, so it runs
    /// under the native lock like every other configuration write.
    pub(crate) fn swe_set_jpl_file(fname: *const c_char);

    /// `double swe_get_tid_acc(void)` — tidal acceleration of the Moon in
    /// arcsec/cy² currently used by `swe_deltat`/`swe_deltat_ex`. Pure read
    /// of shared native configuration; held under the lock with its use.
    pub(crate) fn swe_get_tid_acc() -> c_double;

    /// `void swe_set_tid_acc(double t_acc)` — set the tidal acceleration in
    /// arcsec/cy², or `SE_TIDAL_AUTOMATIC` (999999) to restore the default.
    /// Pure value write to shared native configuration.
    pub(crate) fn swe_set_tid_acc(t_acc: c_double);

    /// `void swe_set_delta_t_userdef(double dt)` — fix Delta T (TT minus UT,
    /// days) returned by `swe_deltat`/`swe_deltat_ex`, or
    /// `SE_DELTAT_AUTOMATIC` (-1E-10) to resume computed values. There is no
    /// native getter: the override state is write-only natively.
    pub(crate) fn swe_set_delta_t_userdef(dt: c_double);

    /// `void swe_set_ephe_path(const char *path)` — copies the path into
    /// native storage (verified against the pinned `sweph.c` setter), so
    /// temporaries are safe. A null pointer selects the default path.
    pub(crate) fn swe_set_ephe_path(path: *const c_char);

    /// `void swe_set_topo(double geolon, double geolat, double geoalt)` —
    /// observer longitude (east positive, degrees), latitude (degrees) and
    /// altitude (meters).
    pub(crate) fn swe_set_topo(geolon: c_double, geolat: c_double, geoalt: c_double);

    /// `void swe_set_sid_mode(int32 sid_mode, double t0, double ayan_t0)`.
    pub(crate) fn swe_set_sid_mode(sid_mode: i32, t0: c_double, ayan_t0: c_double);

    /// `double swe_get_ayanamsa_ut(double tjd_ut)` — ayanamsha in degrees
    /// for the active sidereal mode; no failure mode.
    pub(crate) fn swe_get_ayanamsa_ut(tjd_ut: c_double) -> c_double;

    /// `double swe_get_ayanamsa(double tjd_et)` — ayanamsha in degrees
    /// for Ephemeris Time; no failure mode.
    pub(crate) fn swe_get_ayanamsa(tjd_et: c_double) -> c_double;

    /// `int32 swe_get_ayanamsa_ex(double tjd_et, int32 iflag, double *daya,
    /// char *serr)` — ayanamsha in degrees with an explicit ephemeris
    /// flag; `daya` must address one writable slot, `serr` needs
    /// [`SERR_LEN`] bytes. Returns the (possibly adjusted) flag set on
    /// success, a negative value on failure.
    pub(crate) fn swe_get_ayanamsa_ex(
        tjd_et: c_double,
        iflag: i32,
        daya: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_get_ayanamsa_ex_ut(double tjd_ut, int32 iflag,
    /// double *daya, char *serr)` — same contract as
    /// `swe_get_ayanamsa_ex` for a Universal Time input.
    pub(crate) fn swe_get_ayanamsa_ex_ut(
        tjd_ut: c_double,
        iflag: i32,
        daya: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `const char *swe_get_ayanamsa_name(int32 isidmode)` — short name of
    /// a sidereal mode, or null when the mode has no predefined name.
    /// The returned text aliases native static storage; it is copied
    /// into owned Rust storage before the lock is released.
    ///
    /// Callers must pass a non-negative mode: the native lookup indexes
    /// its name table directly after folding modulo 256, so a negative
    /// value is rejected by the safe wrapper before any native call.
    pub(crate) fn swe_get_ayanamsa_name(isidmode: i32) -> *const c_char;

    /// `char *swe_get_planet_name(int ipl, char *spname)` — writes the body
    /// name into the caller buffer ([`TEXT_BUF_LEN`] bytes) and returns it.
    pub(crate) fn swe_get_planet_name(ipl: c_int, spname: *mut c_char) -> *mut c_char;

    /// `int swe_houses(double tjd_ut, double geolat, double geolon,
    /// int hsys, double *cusps, double *ascmc)` — `cusps` needs
    /// [`CUSPS_NATIVE_LEN`] slots, `ascmc` needs [`ASCMC_NATIVE_LEN`].
    /// Returns `OK` (0) or `ERR` (-1); there is no diagnostic buffer.
    pub(crate) fn swe_houses(
        tjd_ut: c_double,
        geolat: c_double,
        geolon: c_double,
        hsys: c_int,
        cusps: *mut c_double,
        ascmc: *mut c_double,
    ) -> c_int;

    /// `int32 swe_houses_ex(double tjd_ut, int32 iflag, double geolat,
    /// double geolon, int hsys, double *cusps, double *ascmc)` — same
    /// buffers as `swe_houses`, with ephemeris flags for systems that need
    /// an internal solar position.
    pub(crate) fn swe_houses_ex(
        tjd_ut: c_double,
        iflag: i32,
        geolat: c_double,
        geolon: c_double,
        hsys: c_int,
        cusps: *mut c_double,
        ascmc: *mut c_double,
    ) -> i32;

    /// `int swe_houses_ex2(double tjd_ut, int32 iflag, double geolat,
    /// double geolon, int hsys, double *cusp, double *ascmc,
    /// double *cusp_speed, double *ascmc_speed, char *serr)` — house
    /// cusps, angles and their daily speeds. `cusp`/`cusp_speed` need
    /// [`CUSPS_NATIVE_LEN`] slots for twelve-house systems or
    /// [`CUSPS_GAUQUELIN_LEN`] slots for the Gauquelin (`'G'`) sector
    /// layout; `ascmc`/`ascmc_speed` need [`ASCMC_NATIVE_LEN`] slots and
    /// `serr` needs [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_houses_ex2(
        tjd_ut: c_double,
        iflag: i32,
        geolat: c_double,
        geolon: c_double,
        hsys: c_int,
        cusp: *mut c_double,
        ascmc: *mut c_double,
        cusp_speed: *mut c_double,
        ascmc_speed: *mut c_double,
        serr: *mut c_char,
    ) -> c_int;

    /// `int swe_houses_armc(double armc, double geolat, double eps,
    /// int hsys, double *cusp, double *ascmc)` — house cusps and angles
    /// from an explicit ARMC (right ascension of the MC, degrees) and
    /// true obliquity `eps` (degrees) instead of a Julian Day. Same
    /// twelve-house buffers as `swe_houses`; returns `OK` (0) or
    /// `ERR` (-1) with no diagnostic buffer. For the Sunshine system
    /// (`'I'`) the zeroed `ascmc[9]` slot reads as a 0° solar
    /// declination; use `swe_houses_armc_ex2` to supply one.
    pub(crate) fn swe_houses_armc(
        armc: c_double,
        geolat: c_double,
        eps: c_double,
        hsys: c_int,
        cusp: *mut c_double,
        ascmc: *mut c_double,
    ) -> c_int;

    /// `int swe_houses_armc_ex2(double armc, double geolat, double eps,
    /// int hsys, double *cusp, double *ascmc, double *cusp_speed,
    /// double *ascmc_speed, char *serr)` — `swe_houses_armc` plus daily
    /// speeds and diagnostics. Speed buffers share the widths of their
    /// position counterparts; `serr` needs [`SERR_LEN`] bytes. For the
    /// Sunshine system (`'I'`) `ascmc[9]` is read as the solar
    /// declination input on entry and rewritten with it on exit; every
    /// other system ignores that slot.
    pub(crate) fn swe_houses_armc_ex2(
        armc: c_double,
        geolat: c_double,
        eps: c_double,
        hsys: c_int,
        cusp: *mut c_double,
        ascmc: *mut c_double,
        cusp_speed: *mut c_double,
        ascmc_speed: *mut c_double,
        serr: *mut c_char,
    ) -> c_int;

    /// `double swe_house_pos(double armc, double geolat, double eps,
    /// int hsys, double *xpin, char *serr)` — house (or, for `'G'`,
    /// Gauquelin sector) position of an ecliptic point `xpin =
    /// [longitude, latitude]` in degrees. Returns a fractional number
    /// whose integer part is the 1-based house (1-12, plus the fraction
    /// inside it) or sector (1-36); Koch circumpolar failure returns
    /// exactly `0.0`. Diagnostics such as the circumpolar note go to
    /// `serr` ([`SERR_LEN`] bytes, may be null natively but always
    /// provided here). There is no error return: `0.0` with a diagnostic
    /// is the native failure state and is preserved as data.
    pub(crate) fn swe_house_pos(
        armc: c_double,
        geolat: c_double,
        eps: c_double,
        hsys: c_int,
        xpin: *const c_double,
        serr: *mut c_char,
    ) -> c_double;

    /// `const char *swe_house_name(int hsys)` — short name of a house
    /// system (for example `"Placidus"`); unknown selectors fall through
    /// to `"Placidus"` natively. Pure table lookup over static storage;
    /// the result is copied into owned Rust storage immediately.
    pub(crate) fn swe_house_name(hsys: c_int) -> *const c_char;

    /// `int32 swe_gauquelin_sector(double t_ut, int32 ipl, char *starname,
    /// int32 iflag, int32 imeth, double *geopos, double atpress,
    /// double attemp, double *dgsect, char *serr)` — Gauquelin sector
    /// position (1-36 plus the fraction inside the sector) of a body.
    /// `starname` selects a fixed star when non-null and non-empty and
    /// is ignored otherwise (the body number `ipl` then applies); the
    /// safe wrapper always passes a caller-owned 256-byte buffer, so any
    /// native read stays in bounds. `geopos` holds three doubles
    /// (eastern longitude degrees, latitude degrees, altitude meters);
    /// `dgsect` must address one writable slot and `serr` needs
    /// [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_gauquelin_sector(
        t_ut: c_double,
        ipl: i32,
        starname: *mut c_char,
        iflag: i32,
        imeth: i32,
        geopos: *const c_double,
        atpress: c_double,
        attemp: c_double,
        dgsect: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_fixstar(char *star, double tjd, int32 iflag, double *xx,
    /// char *serr)` — position of a fixed star for Ephemeris Time.
    /// `star` is an in/out buffer of [`STAR_BUF_LEN`] bytes: the caller
    /// writes the search string there and the engine overwrites it with
    /// the resolved full name. `xx` needs [`POSITION_LEN`] slots, `serr`
    /// needs [`SERR_LEN`] bytes. Returns the (possibly adjusted) flag set
    /// on success, a negative value on failure (unknown star, missing
    /// catalog data).
    pub(crate) fn swe_fixstar(
        star: *mut c_char,
        tjd_et: c_double,
        iflag: i32,
        xx: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_fixstar_ut(char *star, double tjd_ut, int32 iflag,
    /// double *xx, char *serr)` — same contract as `swe_fixstar` for a
    /// Universal Time input.
    pub(crate) fn swe_fixstar_ut(
        star: *mut c_char,
        tjd_ut: c_double,
        iflag: i32,
        xx: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_fixstar_mag(char *star, double *mag, char *serr)` —
    /// visual magnitude of a fixed star without a position computation.
    /// `star` follows the same in/out [`STAR_BUF_LEN`]-byte contract as
    /// `swe_fixstar`; `mag` must address one writable slot, `serr` needs
    /// [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_fixstar_mag(star: *mut c_char, mag: *mut c_double, serr: *mut c_char) -> i32;

    /// `int32 swe_fixstar2(char *star, double tjd, int32 iflag, double *xx,
    /// char *serr)` — `swe_fixstar` with the extended search semantics:
    /// the query may be a traditional name, a `",nomenclature"` lookup, a
    /// sequential catalog number, or a trailing-`'%'` prefix wildcard. The
    /// resolved full name (`"Name,Nomenclature"`) is written back into the
    /// [`STAR_BUF_LEN`]-byte `star` buffer. Buffer and return contracts
    /// match `swe_fixstar`.
    pub(crate) fn swe_fixstar2(
        star: *mut c_char,
        tjd_et: c_double,
        iflag: i32,
        xx: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_fixstar2_ut(char *star, double tjd_ut, int32 iflag,
    /// double *xx, char *serr)` — same contract as `swe_fixstar2` for a
    /// Universal Time input.
    pub(crate) fn swe_fixstar2_ut(
        star: *mut c_char,
        tjd_ut: c_double,
        iflag: i32,
        xx: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_fixstar2_mag(char *star, double *mag, char *serr)` —
    /// `swe_fixstar_mag` with the `swe_fixstar2` extended search
    /// semantics. Buffer and return contracts match `swe_fixstar_mag`.
    pub(crate) fn swe_fixstar2_mag(star: *mut c_char, mag: *mut c_double, serr: *mut c_char)
    -> i32;

    /// `int32 swe_utc_to_jd(int32 y, int32 m, int32 d, int32 h, int32 min,
    /// double sec, int32 gregflag, double *dret, char *serr)` — convert a
    /// UTC calendar date to Julian Days; `dret` needs 2 slots (`[jd_et,
    /// jd_ut]`), `serr` needs [`SERR_LEN`] bytes. Returns `OK` (0) or
    /// `ERR` (-1); before 1972 the input is treated as UT1.
    pub(crate) fn swe_utc_to_jd(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: c_double,
        gregflag: i32,
        dret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `void swe_jdet_to_utc(double tjd_et, int32 gregflag, ...)` — split
    /// an Ephemeris Time Julian Day into calendar fields. All out
    /// pointers must be valid; there is no failure mode.
    pub(crate) fn swe_jdet_to_utc(
        tjd_et: c_double,
        gregflag: i32,
        iyear: *mut i32,
        imonth: *mut i32,
        iday: *mut i32,
        ihour: *mut i32,
        imin: *mut i32,
        dsec: *mut c_double,
    );

    /// `void swe_jdut1_to_utc(double tjd_ut, int32 gregflag, ...)` —
    /// same layout as `swe_jdet_to_utc` for a Universal Time input.
    pub(crate) fn swe_jdut1_to_utc(
        tjd_ut: c_double,
        gregflag: i32,
        iyear: *mut i32,
        imonth: *mut i32,
        iday: *mut i32,
        ihour: *mut i32,
        imin: *mut i32,
        dsec: *mut c_double,
    );

    /// `void swe_utc_time_zone(...)` — shift a UTC calendar date by
    /// `timezone_hours` (fractional hours, east positive). Pure calendar
    /// arithmetic; all out pointers must be valid.
    pub(crate) fn swe_utc_time_zone(
        iyear: i32,
        imonth: i32,
        iday: i32,
        ihour: i32,
        imin: i32,
        dsec: c_double,
        timezone_hours: c_double,
        iyear_out: *mut i32,
        imonth_out: *mut i32,
        iday_out: *mut i32,
        ihour_out: *mut i32,
        imin_out: *mut i32,
        dsec_out: *mut c_double,
    );

    /// `int swe_date_conversion(int y, int m, int d, double utime,
    /// char c, double *tjd)` — convert a calendar date to a Julian Day;
    /// `c` is `b'g'` or `b'j'`. Returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_date_conversion(
        year: c_int,
        month: c_int,
        day: c_int,
        hour: c_double,
        calendar: c_char,
        tjd: *mut c_double,
    ) -> c_int;

    /// `int swe_day_of_week(double jd)` — weekday of a Julian Day
    /// (0 = Monday per the native convention).
    pub(crate) fn swe_day_of_week(jd: c_double) -> c_int;

    /// `double swe_deltat(double tjd)` — Delta T (TT minus UT) in days,
    /// using the natively configured tidal acceleration and ephemeris
    /// guess. No failure mode; reads shared native configuration.
    pub(crate) fn swe_deltat(tjd: c_double) -> c_double;

    /// `double swe_deltat_ex(double tjd, int32 iflag, char *serr)` —
    /// Delta T in days with an explicit ephemeris-selection flag;
    /// `serr` needs [`SERR_LEN`] bytes and may carry a warning (for
    /// example a missing-data notice) alongside a usable value.
    pub(crate) fn swe_deltat_ex(tjd: c_double, iflag: i32, serr: *mut c_char) -> c_double;

    /// `int32 swe_time_equ(double tjd, double *te, char *serr)` —
    /// equation of time in days; `te` must address one writable slot and
    /// `serr` needs [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_time_equ(tjd: c_double, te: *mut c_double, serr: *mut c_char) -> i32;

    /// `int32 swe_lmt_to_lat(double tjd_lmt, double geolon, double *tjd_lat,
    /// char *serr)` and `swe_lat_to_lmt` — convert between local mean time
    /// and local apparent time. Each returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_lmt_to_lat(
        tjd_lmt: c_double,
        geolon: c_double,
        tjd_lat: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lat_to_lmt(double tjd_lat, double geolon, double *tjd_lmt,
    /// char *serr)` — converse of `swe_lmt_to_lat`.
    pub(crate) fn swe_lat_to_lmt(
        tjd_lat: c_double,
        geolon: c_double,
        tjd_lmt: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `double swe_sidtime(double tjd_ut)` — mean sidereal time at
    /// Greenwich in hours (0-24). Reads the native Delta T state.
    pub(crate) fn swe_sidtime(tjd_ut: c_double) -> c_double;

    /// `double swe_sidtime0(double tjd_ut, double eps, double nut)` —
    /// sidereal time in hours for an explicit obliquity `eps` (degrees)
    /// and nutation in longitude `nut` (degrees).
    pub(crate) fn swe_sidtime0(tjd_ut: c_double, eps: c_double, nut: c_double) -> c_double;

    /// `int32 swe_nod_aps(double tjd_et, int32 ipl, int32 iflag,
    /// int32 method, double *xnasc, double *xndsc, double *xperi,
    /// double *xaphe, char *serr)` — planetary nodes and apsides for
    /// Ephemeris Time. Each of `xnasc`/`xndsc`/`xperi`/`xaphe` needs
    /// [`POSITION_LEN`] slots (ecliptic longitude/latitude/distance plus
    /// daily rates when speeds were requested); `serr` needs [`SERR_LEN`]
    /// bytes. Returns the (possibly adjusted) flag set on success, a
    /// negative value on failure. With `SE_NODBIT_FOPOINT` the focal point
    /// is returned in the `xaphe` slot.
    pub(crate) fn swe_nod_aps(
        tjd_et: c_double,
        ipl: i32,
        iflag: i32,
        method: i32,
        xnasc: *mut c_double,
        xndsc: *mut c_double,
        xperi: *mut c_double,
        xaphe: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_nod_aps_ut(double tjd_ut, ...)` — same contract as
    /// `swe_nod_aps` for a Universal Time input.
    pub(crate) fn swe_nod_aps_ut(
        tjd_ut: c_double,
        ipl: i32,
        iflag: i32,
        method: i32,
        xnasc: *mut c_double,
        xndsc: *mut c_double,
        xperi: *mut c_double,
        xaphe: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_rise_trans(double tjd_ut, int32 ipl, char *starname,
    /// int32 epheflag, int32 rsmi, double *geopos, double atpress,
    /// double attemp, double *tret, char *serr)` — next rising, setting or
    /// meridian transit after `tjd_ut` (UT). `starname` selects a fixed
    /// star when non-null and non-empty and is ignored otherwise (the body
    /// number `ipl` then applies); the safe wrapper passes null for the
    /// body path and a caller-owned NUL-terminated buffer otherwise.
    /// `geopos` holds three doubles (eastern longitude degrees, latitude
    /// degrees, altitude meters); `atpress` (mbar, 0 = estimate from
    /// altitude) and `attemp` (°C) feed refraction; `tret` must address
    /// one writable slot receiving the event time (UT Julian Day) and
    /// `serr` needs [`SERR_LEN`] bytes. Returns 0 when the event was
    /// found, -2 when a rise/set has no event (circumpolar object;
    /// distinct from failure), and -1 (`ERR`) on error.
    pub(crate) fn swe_rise_trans(
        tjd_ut: c_double,
        ipl: i32,
        starname: *mut c_char,
        epheflag: i32,
        rsmi: i32,
        geopos: *const c_double,
        atpress: c_double,
        attemp: c_double,
        tret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_rise_trans_true_hor(..., double horhgt, double *tret,
    /// char *serr)` — `swe_rise_trans` for a local horizon of apparent
    /// height `horhgt` (degrees) at the rise/set point. Buffer and return
    /// contracts match `swe_rise_trans`.
    pub(crate) fn swe_rise_trans_true_hor(
        tjd_ut: c_double,
        ipl: i32,
        starname: *mut c_char,
        epheflag: i32,
        rsmi: i32,
        geopos: *const c_double,
        atpress: c_double,
        attemp: c_double,
        horhgt: c_double,
        tret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `void swe_azalt(double tjd_ut, int32 calc_flag, double *geopos,
    /// double atpress, double attemp, double *xin, double *xaz)` —
    /// celestial-to-horizontal conversion for Universal Time. `calc_flag`
    /// selects the input frame (`SE_ECL2HOR` ecliptic, `SE_EQU2HOR`
    /// equatorial); `geopos` holds three readable doubles (eastern
    /// longitude degrees, latitude degrees, altitude meters); `xin` holds
    /// the input pair (longitude/right ascension, latitude/declination,
    /// degrees) plus an ignored third slot; `xaz` receives three slots
    /// (azimuth from South westward, true altitude, apparent altitude, all
    /// degrees). No failure mode and no diagnostic buffer: inputs are
    /// validated by the safe wrapper and engine-defined pressure handling
    /// is preserved verbatim.
    pub(crate) fn swe_azalt(
        tjd_ut: c_double,
        calc_flag: i32,
        geopos: *const c_double,
        atpress: c_double,
        attemp: c_double,
        xin: *const c_double,
        xaz: *mut c_double,
    );

    /// `void swe_azalt_rev(double tjd_ut, int32 calc_flag, double *geopos,
    /// double *xin, double *xout)` — horizontal-to-celestial conversion
    /// for Universal Time. `calc_flag` selects the output frame
    /// (`SE_HOR2ECL` ecliptic, `SE_HOR2EQU` equatorial); `xin` holds the
    /// input pair (azimuth from South westward, true altitude, degrees)
    /// plus an ignored third slot; `xout` receives exactly
    /// [`CELESTIAL_LEN`] slots. No failure mode; same validation contract
    /// as `swe_azalt`.
    pub(crate) fn swe_azalt_rev(
        tjd_ut: c_double,
        calc_flag: i32,
        geopos: *const c_double,
        xin: *const c_double,
        xout: *mut c_double,
    );

    /// `double swe_refrac(double inalt, double atpress, double attemp,
    /// int32 calc_flag)` — refraction between true and apparent altitude
    /// (degrees). `calc_flag` selects the direction (`SE_TRUE_TO_APP`,
    /// `SE_APP_TO_TRUE`); zero pressure disables the correction (the
    /// input is returned unchanged). Pure function of its arguments; no
    /// buffers, no failure mode.
    pub(crate) fn swe_refrac(
        inalt: c_double,
        atpress: c_double,
        attemp: c_double,
        calc_flag: i32,
    ) -> c_double;

    /// `double swe_refrac_extended(double inalt, double geoalt, double
    /// atpress, double attemp, double lapse_rate, int32 calc_flag, double
    /// *dret)` — `swe_refrac` with observer altitude `geoalt` (meters)
    /// and an explicit lapse rate (K/m). Returns the converted altitude
    /// (degrees) and writes [`REFRACTION_DETAILS_LEN`] detail slots
    /// (true altitude, apparent altitude, refraction amount, horizon dip —
    /// established by probing the pinned build). No failure mode.
    pub(crate) fn swe_refrac_extended(
        inalt: c_double,
        geoalt: c_double,
        atpress: c_double,
        attemp: c_double,
        lapse_rate: c_double,
        calc_flag: i32,
        dret: *mut c_double,
    ) -> c_double;

    /// `void swe_set_lapse_rate(double lapse_rate)` — override the
    /// process-global atmospheric lapse rate (K/m) used by
    /// elevation-dependent native steps. There is no native getter: the
    /// override state is write-only natively, so callers track the active
    /// value themselves. Runs under the native lock like every other
    /// configuration write.
    pub(crate) fn swe_set_lapse_rate(lapse_rate: c_double);

    /// `int32 swe_pheno(double tjd, int32 ipl, int32 iflag, double *attr,
    /// char *serr)` — phase angle, illuminated fraction, elongation,
    /// apparent diameter and magnitude for Ephemeris Time. `attr` needs
    /// [`PHENO_LEN`] slots, `serr` needs [`SERR_LEN`] bytes. Returns the
    /// (possibly adjusted) flag set on success, a negative value on
    /// failure (unknown body, missing elements).
    pub(crate) fn swe_pheno(
        tjd_et: c_double,
        ipl: i32,
        iflag: i32,
        attr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_pheno_ut(double tjd_ut, int32 ipl, int32 iflag, double
    /// *attr, char *serr)` — same contract as `swe_pheno` for a Universal
    /// Time input.
    pub(crate) fn swe_pheno_ut(
        tjd_ut: c_double,
        ipl: i32,
        iflag: i32,
        attr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_get_orbital_elements(double tjd_et, int32 ipl, int32
    /// iflag, double *dret, char *serr)` — osculating Keplerian elements
    /// for Ephemeris Time. There is no UT variant natively. The engine
    /// writes the 17 element slots 0-16 (established by probing); the
    /// safe wrapper passes a zero-initialized [`ORBEL_LEN`]-slot buffer
    /// so the reserved tail is deterministic. Returns `OK` (0) on
    /// success, `ERR` (-1) on failure (unsupported body, missing data).
    pub(crate) fn swe_get_orbital_elements(
        tjd_et: c_double,
        ipl: i32,
        iflag: i32,
        dret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_orbit_max_min_true_distance(double tjd_et, int32 ipl,
    /// int32 iflag, double *dmax, double *dmin, double *dtrue, char
    /// *serr)` — extreme diurnal geocentric distances (AU) plus the
    /// current true distance for Ephemeris Time. Each of
    /// `dmax`/`dmin`/`dtrue` must address one writable slot; `serr` needs
    /// [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR` (-1).
    pub(crate) fn swe_orbit_max_min_true_distance(
        tjd_et: c_double,
        ipl: i32,
        iflag: i32,
        dmax: *mut c_double,
        dmin: *mut c_double,
        dtrue: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `double swe_solcross(double x2cross, double jd_et, int32 flag,
    /// char *serr)` — Julian Day (Ephemeris Time) of the next crossing of
    /// the Sun over ecliptic longitude `x2cross` (degrees) after `jd_et`.
    /// The engine always searches forward. `serr` needs [`SERR_LEN`]
    /// bytes. Errors are reported by returning a Julian Day below
    /// `jd_et` (`jd_et - 1`); there is no other status channel.
    pub(crate) fn swe_solcross(
        x2cross: c_double,
        jd_et: c_double,
        flag: i32,
        serr: *mut c_char,
    ) -> c_double;

    /// `double swe_solcross_ut(double x2cross, double jd_ut, int32 flag,
    /// char *serr)` — same contract as `swe_solcross` for a Universal
    /// Time input and result.
    pub(crate) fn swe_solcross_ut(
        x2cross: c_double,
        jd_ut: c_double,
        flag: i32,
        serr: *mut c_char,
    ) -> c_double;

    /// `double swe_mooncross(double x2cross, double jd_et, int32 flag,
    /// char *serr)` — Julian Day (Ephemeris Time) of the next crossing of
    /// the Moon over ecliptic longitude `x2cross` (degrees) after `jd_et`.
    /// Same forward-only search and below-input error sentinel as
    /// `swe_solcross`.
    pub(crate) fn swe_mooncross(
        x2cross: c_double,
        jd_et: c_double,
        flag: i32,
        serr: *mut c_char,
    ) -> c_double;

    /// `double swe_mooncross_ut(double x2cross, double jd_ut, int32 flag,
    /// char *serr)` — same contract as `swe_mooncross` for a Universal
    /// Time input and result.
    pub(crate) fn swe_mooncross_ut(
        x2cross: c_double,
        jd_ut: c_double,
        flag: i32,
        serr: *mut c_char,
    ) -> c_double;

    /// `double swe_mooncross_node(double jd_et, int32 flag, double *xlon,
    /// double *xlat, char *serr)` — Julian Day (Ephemeris Time) of the
    /// next crossing of the Moon over its own orbital node (latitude
    /// zero) after `jd_et`, either ascending or descending. `xlon`/`xlat`
    /// must each address one writable slot receiving the ecliptic
    /// longitude/latitude (degrees) at the crossing; `serr` needs
    /// [`SERR_LEN`] bytes. Same below-input error sentinel as
    /// `swe_solcross`.
    pub(crate) fn swe_mooncross_node(
        jd_et: c_double,
        flag: i32,
        xlon: *mut c_double,
        xlat: *mut c_double,
        serr: *mut c_char,
    ) -> c_double;

    /// `double swe_mooncross_node_ut(double jd_ut, int32 flag, double
    /// *xlon, double *xlat, char *serr)` — same contract as
    /// `swe_mooncross_node` for a Universal Time input and result.
    pub(crate) fn swe_mooncross_node_ut(
        jd_ut: c_double,
        flag: i32,
        xlon: *mut c_double,
        xlat: *mut c_double,
        serr: *mut c_char,
    ) -> c_double;

    /// `int32 swe_helio_cross(int32 ipl, double x2cross, double jd_et,
    /// int32 iflag, int32 dir, double *jd_cross, char *serr)` — Julian Day
    /// (Ephemeris Time) of the next (`dir >= 0`) or previous (`dir < 0`)
    /// crossing of body `ipl` over heliocentric ecliptic longitude
    /// `x2cross` (degrees). `jd_cross` must address one writable slot;
    /// `serr` needs [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR` (-1):
    /// the Sun, the Moon, nodes/apsides and out-of-range numbers are
    /// rejected natively with a diagnostic.
    pub(crate) fn swe_helio_cross(
        ipl: i32,
        x2cross: c_double,
        jd_et: c_double,
        iflag: i32,
        dir: i32,
        jd_cross: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_helio_cross_ut(int32 ipl, double x2cross, double jd_ut,
    /// int32 iflag, int32 dir, double *jd_cross, char *serr)` — same
    /// contract as `swe_helio_cross` for a Universal Time input and
    /// result.
    pub(crate) fn swe_helio_cross_ut(
        ipl: i32,
        x2cross: c_double,
        jd_ut: c_double,
        iflag: i32,
        dir: i32,
        jd_cross: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_sol_eclipse_when_glob(double tjd_start, int32 ifl, int32
    /// ifltype, double *tret, int32 backward, char *serr)` — next
    /// (`backward == 0`) or previous (nonzero) global solar eclipse after
    /// `tjd_start` (Universal Time). `ifltype` filters the wanted eclipse
    /// types (`SE_ECL_*` bits, 0 = any); `tret` needs
    /// [`ECLIPSE_TRET_LEN`] slots (maximum plus successive
    /// contact/center-line times, inapplicable entries 0.0); `serr` needs
    /// [`SERR_LEN`] bytes. Returns the found eclipse-type bitmask
    /// (`SE_ECL_TOTAL`, `SE_ECL_PARTIAL`, ...), or `ERR` (-1) on failure
    /// (for example an impossible type combination such as central +
    /// partial).
    pub(crate) fn swe_sol_eclipse_when_glob(
        tjd_start: c_double,
        ifl: i32,
        ifltype: i32,
        tret: *mut c_double,
        backward: c_int,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lun_eclipse_when(double tjd_start, int32 ifl, int32
    /// ifltype, double *tret, int32 backward, char *serr)` — next or
    /// previous global lunar eclipse after `tjd_start` (Universal Time).
    /// Buffer and return contracts match `swe_sol_eclipse_when_glob`,
    /// with the lunar type bits (`SE_ECL_TOTAL`, `SE_ECL_PARTIAL`,
    /// `SE_ECL_PENUMBRAL`); annular-only requests fail natively.
    pub(crate) fn swe_lun_eclipse_when(
        tjd_start: c_double,
        ifl: i32,
        ifltype: i32,
        tret: *mut c_double,
        backward: c_int,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_sol_eclipse_where(double tjd_ut, int32 ifl, double
    /// *geopos, double *attr, char *serr)` — geographic circumstances of
    /// the solar eclipse in progress at `tjd_ut` (Universal Time).
    /// `geopos` receives [`ECLIPSE_GEOPOS_LEN`] slots (eastern longitude,
    /// northern latitude, degrees, of the greatest eclipse); `attr` needs
    /// [`ECLIPSE_ATTR_LEN`] slots (see `swe_sol_eclipse_how` for the
    /// layout); `serr` needs [`SERR_LEN`] bytes. Returns the eclipse-type
    /// bitmask, or 0 when no eclipse is in progress (kept as data with
    /// the diagnostic, not an error), or `ERR` (-1) on failure.
    pub(crate) fn swe_sol_eclipse_where(
        tjd_ut: c_double,
        ifl: i32,
        geopos: *mut c_double,
        attr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_sol_eclipse_how(double tjd_ut, int32 ifl, double *geopos,
    /// double *attr, char *serr)` — local circumstances of the solar
    /// eclipse at `tjd_ut` (Universal Time) for the observer `geopos =
    /// [longitude, latitude, altitude]` (east-positive degrees, degrees
    /// north, meters; altitudes outside −500…25000 m fail natively).
    /// `attr` needs [`ECLIPSE_ATTR_LEN`] slots: magnitude (diameter
    /// fraction), lunar/solar diameter ratio, obscuration, core-shadow
    /// width in km, azimuth, true/apparent altitude, Moon–Sun separation,
    /// NASA magnitude, saros series/member. Returns the local phase
    /// bitmask, or 0 when no eclipse is in progress here (kept as data),
    /// or `ERR` (-1) on failure.
    pub(crate) fn swe_sol_eclipse_how(
        tjd_ut: c_double,
        ifl: i32,
        geopos: *const c_double,
        attr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lun_eclipse_how(double tjd_ut, int32 ifl, double *geopos,
    /// double *attr, char *serr)` — circumstances of the lunar eclipse at
    /// `tjd_ut` (Universal Time). `geopos` is either null (geocentric
    /// magnitudes, no azimuth/altitude) or `[longitude, latitude,
    /// altitude]` like `swe_sol_eclipse_how` (same altitude domain);
    /// `attr` needs [`ECLIPSE_ATTR_LEN`] slots: umbral/penumbral
    /// magnitude, azimuth, true/apparent lunar altitude, distance from
    /// opposition, saros series/member. Returns the eclipse-type bitmask,
    /// or 0 when no (visible) eclipse is in progress (kept as data), or
    /// `ERR` (-1) on failure.
    pub(crate) fn swe_lun_eclipse_how(
        tjd_ut: c_double,
        ifl: i32,
        geopos: *const c_double,
        attr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_sol_eclipse_when_loc(double tjd_start, int32 ifl, double
    /// *geopos, double *tret, double *attr, int32 backward, char *serr)` —
    /// next solar eclipse visible from the observer `geopos = [longitude,
    /// latitude, altitude]` (east-positive degrees, degrees north, meters;
    /// altitudes outside −500…25000 m fail natively). `tret` needs
    /// [`LOCAL_SOLAR_TRET_LEN`] slots (maximum, four contacts, sunrise,
    /// sunset; inapplicable entries keep the caller value, hence the safe
    /// wrapper zero-initializes); `attr` needs [`ECLIPSE_ATTR_LEN`] slots
    /// with the `swe_sol_eclipse_how` layout at maximum (slots 0-10
    /// meaningful). `backward` selects the search direction (nonzero =
    /// backward); the safe wrapper passes only the direction bit here —
    /// the native `SE_ECL_ONE_TRY` single-conjunction mode is decoded
    /// only by the lunar occultation searches (see
    /// [`crate::OccultSearchOptions`]). Returns the local phase bitmask (`SE_ECL_TOTAL`,
    /// `SE_ECL_PARTIAL`, ... combined with `SE_ECL_VISIBLE` and the
    /// `SE_ECL_*_VISIBLE` contact bits), or `ERR` (-1) on failure. The
    /// search advances lunation by lunation until a visible eclipse is
    /// found; there is no "absent event" return.
    pub(crate) fn swe_sol_eclipse_when_loc(
        tjd_start: c_double,
        ifl: i32,
        geopos: *const c_double,
        tret: *mut c_double,
        attr: *mut c_double,
        backward: c_int,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lun_eclipse_when_loc(double tjd_start, int32 ifl, double
    /// *geopos, double *tret, double *attr, int32 backward, char *serr)` —
    /// next lunar eclipse visible from the observer `geopos` (same layout
    /// and altitude domain as `swe_sol_eclipse_when_loc`; unlike
    /// `swe_lun_eclipse_how` the observer must be non-null because the
    /// search also runs rise/set transits on it). `tret` needs
    /// [`ECLIPSE_TRET_LEN`] slots (maximum, reserved, partial begin/end,
    /// total begin/end, penumbral begin/end, moonrise, moonset);
    /// `attr` needs [`ECLIPSE_ATTR_LEN`] slots with the
    /// `swe_lun_eclipse_how` layout at maximum. Returns the type bitmask
    /// (`SE_ECL_TOTAL`, `SE_ECL_PARTIAL`, `SE_ECL_PENUMBRAL` combined with
    /// `SE_ECL_VISIBLE` and the phase-visibility bits), or `ERR` (-1) on
    /// failure. The search advances eclipse by eclipse until a visible one
    /// is found; there is no "absent event" return.
    pub(crate) fn swe_lun_eclipse_when_loc(
        tjd_start: c_double,
        ifl: i32,
        geopos: *const c_double,
        tret: *mut c_double,
        attr: *mut c_double,
        backward: c_int,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lun_occult_when_glob(double tjd_start, int32 ipl, char
    /// *starname, int32 ifl, int32 ifltype, double *tret, int32 backward,
    /// char *serr)` — next (`backward == 0`) or previous (nonzero) lunar
    /// occultation visible somewhere on Earth. `ipl` selects the occulted
    /// planet, `starname` a fixed star when non-null and non-empty (the
    /// body number then applies); the safe wrapper always passes a
    /// caller-owned 256-byte buffer, so any native read — and the
    /// `swe_fixstar` resolved-name write-back — stays in bounds. Negative
    /// body numbers fold to the Sun natively. `ifltype` filters the wanted
    /// types (`SE_ECL_*` bits, 0 = any; central + partial fails natively,
    /// annular fails natively for non-solar targets); `tret` needs
    /// [`ECLIPSE_TRET_LEN`] slots with the global-eclipse layout;
    /// `serr` needs [`SERR_LEN`] bytes. `backward` carries the direction
    /// in bit 0 and the `SE_ECL_ONE_TRY` single-conjunction mode in its
    /// dedicated bit (see [`crate::OccultSearchOptions`]); the safe
    /// wrapper encodes both via `encode_occult_backward` and never sets
    /// any other bit. A one-try miss returns 0 with `tret[0]` holding the
    /// examined conjunction epoch. Returns the occultation-type bitmask,
    /// or `ERR` (-1) on failure (impossible type combination,
    /// unoccultable star, failed position computation). Without
    /// `SE_ECL_ONE_TRY` the search runs until an event is found; there is
    /// no "absent event" return.
    pub(crate) fn swe_lun_occult_when_glob(
        tjd_start: c_double,
        ipl: i32,
        starname: *mut c_char,
        ifl: i32,
        ifltype: i32,
        tret: *mut c_double,
        backward: c_int,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lun_occult_when_loc(double tjd_start, int32 ipl, char
    /// *starname, int32 ifl, double *geopos, double *tret, double *attr,
    /// int32 backward, char *serr)` — next lunar occultation visible from
    /// the observer `geopos` (same layout and altitude domain as
    /// `swe_sol_eclipse_when_loc`). Body/star selection matches
    /// `swe_lun_occult_when_glob`. `tret` is zeroed over ten slots by the
    /// engine itself, so the safe wrapper passes [`ECLIPSE_TRET_LEN`]
    /// slots; slots 0-6 hold maximum, four contacts, rise, set of the
    /// occulted body and slots 7-9 stay 0.0 (reserved). `attr` needs
    /// [`ECLIPSE_ATTR_LEN`] slots with the occultation layout at maximum
    /// (slots 0-7 meaningful; slots 8-10, the solar NASA/saros fields,
    /// stay 0.0 for non-solar targets). Returns the local occultation
    /// bitmask (`SE_ECL_TOTAL`, `SE_ECL_PARTIAL` combined with
    /// `SE_ECL_VISIBLE`, the `SE_ECL_*_VISIBLE` contact bits and the
    /// `SE_ECL_OCC_*_DAYLIGHT` bits), or `ERR` (-1) on failure. With a
    /// one-try `backward` encoding (see [`crate::OccultSearchOptions`])
    /// a miss returns 0 with `tret[0]` holding a date suitable for the
    /// next try; without `SE_ECL_ONE_TRY` the search runs until a visible
    /// event is found.
    pub(crate) fn swe_lun_occult_when_loc(
        tjd_start: c_double,
        ipl: i32,
        starname: *mut c_char,
        ifl: i32,
        geopos: *const c_double,
        tret: *mut c_double,
        attr: *mut c_double,
        backward: c_int,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_lun_occult_where(double tjd, int32 ipl, char *starname,
    /// int32 ifl, double *geopos, double *attr, char *serr)` —
    /// geographic circumstances of the lunar occultation in progress at
    /// `tjd` (Universal Time). Body/star selection matches
    /// `swe_lun_occult_when_glob`. `geopos` receives
    /// [`ECLIPSE_GEOPOS_LEN`] slots (eastern longitude, northern latitude,
    /// degrees, of the greatest occultation); `attr` needs
    /// [`ECLIPSE_ATTR_LEN`] slots with the occultation layout (slots 0-7
    /// meaningful, slots 8-10 read 0.0 for non-solar targets);
    /// `serr` needs [`SERR_LEN`] bytes. Returns the occultation-type
    /// bitmask, or 0 when no occultation is in progress (kept as data with
    /// the diagnostic, not an error), or `ERR` (-1) on failure.
    pub(crate) fn swe_lun_occult_where(
        tjd_ut: c_double,
        ipl: i32,
        starname: *mut c_char,
        ifl: i32,
        geopos: *mut c_double,
        attr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_heliacal_ut(double tjdstart_ut, double *geopos, double
    /// *datm, double *dobs, char *ObjectName, int32 TypeEvent, int32 iflag,
    /// double *dret, char *serr)` — next heliacal rising/setting (or, for
    /// the Moon and inner planets, first/last visibility) of a planet or
    /// fixed star after `tjdstart_ut` (Universal Time). `geopos` holds
    /// [`HELIACAL_GEO_LEN`] doubles (eastern longitude degrees, latitude
    /// degrees, eye height meters; heights outside −500…25000 m fail
    /// natively); `datm` holds [`HELIACAL_ATM_LEN`] doubles and `dobs`
    /// holds [`HELIACAL_OBS_LEN`] doubles (the engine writes its defaults
    /// into zero entries, so the safe wrapper passes owned arrays).
    /// `ObjectName` names the planet (`"Venus"`, ...) or star
    /// (`"Sirius"`, ...); the safe wrapper passes a caller-owned
    /// [`TEXT_BUF_LEN`]-byte buffer. `TypeEvent` selects morning-first (1),
    /// evening-last (2), evening-first (3) or morning-last (4); `iflag`
    /// combines the ephemeris source bits with the `SE_HELFLAG_*` search
    /// bits. `dret` needs [`HELIACAL_DRET_LEN`] slots (beginning, optimum,
    /// end of visibility; optimum/end read 0.0 with the arcus-visionis
    /// kinds) and `serr` needs [`SERR_LEN`] bytes. Returns `OK` (0) or
    /// `ERR` (-1); the Sun, impossible type/object combinations and
    /// out-of-range heights fail natively with a diagnostic.
    pub(crate) fn swe_heliacal_ut(
        tjdstart_ut: c_double,
        geopos: *mut c_double,
        datm: *mut c_double,
        dobs: *mut c_double,
        object_name: *mut c_char,
        type_event: i32,
        iflag: i32,
        dret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_heliacal_pheno_ut(double tjd_ut, double *geopos, double
    /// *datm, double *dobs, char *ObjectName, int32 TypeEvent, int32
    /// helflag, double *darr, char *serr)` — heliacal circumstances of a
    /// planet or fixed star at `tjd_ut` (Universal Time).
    /// Observer/atmosphere/object selection matches `swe_heliacal_ut`;
    /// `helflag` combines the ephemeris source bits with the
    /// `SE_HELFLAG_*` display bits. `darr` needs [`HELIACAL_PHENO_LEN`]
    /// slots (28 phenomenon components plus two reserved slots reading
    /// 0.0); `serr` needs [`SERR_LEN`] bytes. Returns `OK` (0) or `ERR`
    /// (-1). Unlike `swe_vis_limit_mag` this call always computes: an
    /// object below the horizon yields negative altitudes as data, never
    /// a dedicated status.
    pub(crate) fn swe_heliacal_pheno_ut(
        tjd_ut: c_double,
        geopos: *mut c_double,
        datm: *mut c_double,
        dobs: *mut c_double,
        object_name: *mut c_char,
        type_event: i32,
        helflag: i32,
        darr: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `int32 swe_vis_limit_mag(double tjdut, double *geopos, double *datm,
    /// double *dobs, char *ObjectName, int32 helflag, double *dret, char
    /// *serr)` — visual limiting magnitude and visibility geometry at
    /// `tjdut` (Universal Time). Observer/atmosphere/object selection
    /// matches `swe_heliacal_ut` (no event type: the instant is given).
    /// `dret` needs [`VISLIM_DRET_LEN`] slots (limiting magnitude,
    /// geometry details, object magnitude); `serr` needs [`SERR_LEN`]
    /// bytes. Returns -1 (`ERR`) on failure (for example the Sun, which
    /// has no limiting magnitude, or an unknown object), -2 when the
    /// object is below the local horizon (kept as data with the
    /// diagnostic, not an error), and otherwise a vision-mode status
    /// (0 photopic, 1 scotopic, plus bit 1 near the photopic/scotopic
    /// limit).
    pub(crate) fn swe_vis_limit_mag(
        tjdut: c_double,
        geopos: *mut c_double,
        datm: *mut c_double,
        dobs: *mut c_double,
        object_name: *mut c_char,
        helflag: i32,
        dret: *mut c_double,
        serr: *mut c_char,
    ) -> i32;

    /// `void swe_close(void)` — release native caches and open files.
    /// Process-wide: it affects every subsequent calculation until the
    /// library lazily re-initializes.
    pub(crate) fn swe_close();

    /// `void swe_cotrans(double *xpo, double *xpn, double eps)` —
    /// rotate one position triple between the equatorial and ecliptic
    /// frames. Both `xpo` (input) and `xpn` (output) address exactly
    /// three slots (longitude/RA, latitude/Dec, distance in mixed
    /// degrees/AU); `eps` is the obliquity in degrees and its sign
    /// selects the direction (established by probing the pinned build:
    /// positive converts equatorial to ecliptic, negative converts
    /// back). Distance passes through unchanged. Pure: no shared state.
    pub(crate) fn swe_cotrans(xpo: *mut c_double, xpn: *mut c_double, eps: c_double);

    /// `void swe_cotrans_sp(double *xpo, double *xpn, double eps)` —
    /// `swe_cotrans` with speed propagation. Both arrays address exactly
    /// six slots (position triple plus the matching daily-rate triple);
    /// direction follows the sign of `eps` as in `swe_cotrans`
    /// (established by probing: all six output slots are written).
    /// Pure: no shared state.
    pub(crate) fn swe_cotrans_sp(xpo: *mut c_double, xpn: *mut c_double, eps: c_double);

    /// `double swe_degnorm(double x)` — normalize degrees into
    /// [0, 360). Pure: no shared state.
    pub(crate) fn swe_degnorm(x: c_double) -> c_double;

    /// `double swe_radnorm(double x)` — normalize radians into
    /// [0, 2π). Pure: no shared state.
    pub(crate) fn swe_radnorm(x: c_double) -> c_double;

    /// `double swe_difdegn(double p1, double p2)` — signed `p1 − p2`
    /// distance in degrees normalized into [0, 360). Pure.
    pub(crate) fn swe_difdegn(p1: c_double, p2: c_double) -> c_double;

    /// `double swe_difdeg2n(double p1, double p2)` — signed `p1 − p2`
    /// distance in degrees folded to ±180. Pure.
    pub(crate) fn swe_difdeg2n(p1: c_double, p2: c_double) -> c_double;

    /// `double swe_difrad2n(double p1, double p2)` — signed `p1 − p2`
    /// distance in radians folded to ±π. Pure.
    pub(crate) fn swe_difrad2n(p1: c_double, p2: c_double) -> c_double;

    /// `double swe_deg_midp(double x1, double x0)` — wrap-aware midpoint
    /// of two degree values (for example 350 and 10 meet at 0, not at
    /// 180). Pure.
    pub(crate) fn swe_deg_midp(x1: c_double, x0: c_double) -> c_double;

    /// `double swe_rad_midp(double x1, double x0)` — wrap-aware midpoint
    /// of two radian values. Pure.
    pub(crate) fn swe_rad_midp(x1: c_double, x0: c_double) -> c_double;

    /// `int32 swe_d2l(double x)` — convert to 32-bit integer with
    /// rounding (halves round away from zero, established by probing:
    /// 2.5 → 3, −2.5 → −3). Out-of-range input is a C-side undefined
    /// conversion, so the safe wrapper validates the domain first.
    pub(crate) fn swe_d2l(x: c_double) -> i32;

    /// `centisec swe_csnorm(centisec p)` — normalize a centisecond angle
    /// into [0, 129600000) (the centisecond equivalent of [0, 360)).
    /// Pure: no shared state.
    pub(crate) fn swe_csnorm(p: i32) -> i32;

    /// `centisec swe_difcsn(centisec p1, centisec p2)` — `p1 − p2`
    /// distance in centiseconds normalized into [0, 129600000). Pure.
    pub(crate) fn swe_difcsn(p1: i32, p2: i32) -> i32;

    /// `centisec swe_difcs2n(centisec p1, centisec p2)` — `p1 − p2`
    /// distance in centiseconds folded to ±64800000 (±180°); an exact
    /// half-turn reports −64800000 (established by probing). Pure.
    pub(crate) fn swe_difcs2n(p1: i32, p2: i32) -> i32;

    /// `centisec swe_csroundsec(centisec x)` — round a centisecond value
    /// to whole arcseconds (multiples of 100). Values just below a 30°
    /// sector boundary round down instead of crossing it (established by
    /// probing: 10799950 → 10799900). Pure.
    pub(crate) fn swe_csroundsec(x: i32) -> i32;

    /// `char *swe_cs2timestr(CSEC t, int sep, AS_BOOL suppressZero,
    /// char *a)` — format centiseconds as `HH<sep>MM<sep>SS` hours. `t`
    /// is centiseconds (1 hour = 360000); `sep` is the field separator
    /// character code; nonzero `suppressZero` drops a zero seconds field
    /// (`"12:34"` instead of `"12:34:00"`). The text is written into the
    /// caller buffer ([`FORMAT_BUF_LEN`] bytes, ample for the 8/5-byte
    /// outputs established by probing) and the buffer pointer is
    /// returned. Negative inputs format engine-defined (established by
    /// probing); callers wrap into [0, 24h) first. Pure: no shared
    /// state.
    pub(crate) fn swe_cs2timestr(
        t: i32,
        sep: c_int,
        suppress_zero: c_int,
        a: *mut c_char,
    ) -> *mut c_char;

    /// `char *swe_cs2lonlatstr(CSEC t, char pchar, char mchar, char *s)`
    /// — format a centisecond angle with a direction character:
    /// `{deg}{pchar}{min:02d}` when the rounded seconds are zero,
    /// `{deg}{pchar}{min:02d}'{sec:02d}` otherwise (established by
    /// probing: 360000 with `N`/`S` gives `"1N00"`). Zero takes the
    /// `pchar` branch; negatives take `mchar` with the absolute value.
    /// Same buffer contract as `swe_cs2timestr`. Pure.
    pub(crate) fn swe_cs2lonlatstr(
        t: i32,
        pchar: c_char,
        mchar: c_char,
        s: *mut c_char,
    ) -> *mut c_char;

    /// `char *swe_cs2degstr(CSEC t, char *a)` — format a centisecond
    /// angle as whole degrees, minutes and whole (truncated) arcseconds
    /// with a UTF-8 degree sign (established by probing: 3723456 gives
    /// `"10°20'34"`, i.e. bytes `31 30 c2 b0 32 30 27 33 34`). Same
    /// buffer contract as `swe_cs2timestr`. Pure.
    pub(crate) fn swe_cs2degstr(t: i32, a: *mut c_char) -> *mut c_char;

    /// `void swe_split_deg(double ddeg, int32 roundflag, int32 *ideg,
    /// int32 *imin, int32 *isec, double *dsecfr, int32 *isgn)` — split a
    /// degree value into degrees, minutes, seconds, fractional seconds
    /// and a sign/segment index. All five out pointers must be valid.
    /// `roundflag` combines the `SE_SPLIT_DEG_*` bits (round to
    /// sec/min/deg, zodiacal 30° segments, nakshatra segments, keep-sign
    /// and keep-degree guards); unknown bits pass through engine-defined
    /// and negative inputs split the absolute value with `isgn = −1`
    /// (except the zodiacal form, established by probing). Pure: no
    /// shared state, but non-finite input is rejected by the safe
    /// wrapper first since the decomposition has no range check.
    pub(crate) fn swe_split_deg(
        ddeg: c_double,
        roundflag: i32,
        ideg: *mut i32,
        imin: *mut i32,
        isec: *mut i32,
        dsecfr: *mut c_double,
        isgn: *mut i32,
    );

    /// `const char *swe_get_current_file_data(int ifno, double *tfstart,
    /// double *tfend, int *denum)` — identity of the ephemeris file
    /// backing file slot `ifno` (0 = planet file, 1 = Moon file). All
    /// three out pointers must be valid. Returns the file path in
    /// native static storage (copied into owned Rust storage before the
    /// lock is released) with the covered Julian-Day range and the JPL
    /// ephemeris number, or null when no file backs the slot — in which
    /// case the out parameters are left untouched (established by
    /// probing: slots 2-4, out-of-range ids and pre-calculation reads
    /// all report null). Reads shared native state: held under the
    /// lock with its use.
    pub(crate) fn swe_get_current_file_data(
        ifno: c_int,
        tfstart: *mut c_double,
        tfend: *mut c_double,
        denum: *mut c_int,
    ) -> *const c_char;
}

/// Read a native text buffer bounded by `capacity`.
///
/// SAFETY: `ptr` must point to `capacity` readable bytes owned by the
/// caller (typically a stack buffer passed to exactly one native call
/// whose results were fully written before this read).
///
/// A missing terminator degrades to a lossy read of the whole buffer;
/// truncation and non-UTF-8 content can never cause an over-read.
pub(crate) unsafe fn read_native_text(ptr: *const c_char, capacity: usize) -> String {
    // SAFETY: the caller guarantees `capacity` readable bytes. Only reads
    // inside that range are performed: either up to the first NUL (a valid
    // C string inside the buffer) or, without a terminator, a bounded
    // slice of exactly `capacity` bytes.
    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), capacity) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(capacity);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Read a native NUL-terminated string returned by pointer.
///
/// SAFETY: `ptr` must be either null or point to a valid NUL-terminated C
/// string whose lifetime covers this call. The contents are copied into an
/// owned `String` immediately; the pointer never escapes.
pub(crate) unsafe fn copy_returned_string(ptr: *const c_char, fallback: &[u8]) -> String {
    if ptr.is_null() {
        return String::from_utf8_lossy(fallback).into_owned();
    }
    // SAFETY: guaranteed by the caller; `CStr` stops at the terminator so
    // no length assumption is needed beyond validity.
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}
