//! Angle normalization, centisecond helpers, formatted angle/time strings,
//! degree splitting and equatorial/ecliptic rotation utilities.
//!
//! Direct wrappers over the linked `swephlib.c` helpers. These are pure
//! computations over their arguments: they read no ephemeris files and
//! depend on no process-global configuration, so they are usable with any
//! source selection. Every function still runs serialized through the
//! process-wide native lock for uniformity. Infallible pure helpers
//! ([`csnorm`], [`degnorm`] and friends) degrade to the same unlocked
//! call when the lock is poisoned rather than failing a computation that
//! touches no shared state; fallible functions report the poison as a
//! classified [`Error`].
//!
//! The rotation direction of [`cotrans`]/[`cotrans_sp`] follows the sign
//! of the obliquity, established by probing the pinned build: a positive
//! `eps` converts equatorial (right ascension, declination) to ecliptic
//! (longitude, latitude), a negative `eps` converts back.

use std::ffi::{c_char, c_double, c_int};

use crate::error::Error;
use crate::ffi;
use crate::ffi::{FORMAT_BUF_LEN, read_native_text};
use crate::state::with_native_access;

/// Rotate one position triple between the equatorial and ecliptic frames.
///
/// `longitude_or_ra`/`latitude_or_dec` are degrees, `distance` is in the
/// same length unit on input and output (astronomical units in
/// ephemeris use; the rotation never rescales it). `eps` is the
/// obliquity of the ecliptic in degrees: positive converts equatorial
/// to ecliptic, negative converts ecliptic to equatorial.
///
/// Returns `(longitude/RA, latitude/Dec, distance)` in degrees. A
/// forward-then-backward round trip restores the input to rounding
/// scale (~1e-14 here).
///
/// Non-finite inputs propagate naturally (no validation is needed: the
/// call performs plain floating-point arithmetic with no shared state).
/// Infallible except for lock poisoning.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::cotrans;
///
/// // The vernal equinox sits at the origin of both frames.
/// let equinox = cotrans(0.0, 0.0, 1.0, 23.44)?;
/// assert!(equinox[0].abs() < 1e-9 && equinox[1].abs() < 1e-9);
/// assert_eq!(equinox[2], 1.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn cotrans(
    longitude_or_ra: f64,
    latitude_or_dec: f64,
    distance: f64,
    eps: f64,
) -> Result<[f64; 3], Error> {
    with_native_access(|| {
        let input = [longitude_or_ra, latitude_or_dec, distance];
        let mut output = [0.0; 3];
        // SAFETY: `input` owns three readable slots and `output` owns
        // three writable slots; both outlive this call under the native
        // lock. The engine writes exactly three slots (established by
        // probing with sentinel-filled buffers).
        unsafe {
            ffi::swe_cotrans(
                input.as_ptr() as *mut c_double,
                output.as_mut_ptr(),
                eps as c_double,
            );
        }
        Ok(output)
    })
}

/// Rotate one position triple with speed propagation between the
/// equatorial and ecliptic frames.
///
/// Same frame/direction contract as [`cotrans`]; the three rate
/// arguments are daily rates matching the position units
/// (degrees/day, degrees/day, distance/day). Returns the rotated
/// position triple and the rotated rate triple.
///
/// Infallible except for lock poisoning.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::cotrans_sp;
///
/// let (pos, speed) = cotrans_sp(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 23.44)?;
/// assert!(pos[0].abs() < 1e-9 && pos[1].abs() < 1e-9);
/// assert_eq!(pos[2], 1.0);
/// assert!(speed[0] > 0.9 && speed[0] < 1.1);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
#[allow(clippy::too_many_arguments)]
pub fn cotrans_sp(
    longitude_or_ra: f64,
    latitude_or_dec: f64,
    distance: f64,
    longitude_or_ra_speed: f64,
    latitude_or_dec_speed: f64,
    distance_speed: f64,
    eps: f64,
) -> Result<([f64; 3], [f64; 3]), Error> {
    with_native_access(|| {
        let input = [
            longitude_or_ra,
            latitude_or_dec,
            distance,
            longitude_or_ra_speed,
            latitude_or_dec_speed,
            distance_speed,
        ];
        let mut output = [0.0; 6];
        // SAFETY: `input` owns six readable slots and `output` owns six
        // writable slots; both outlive this call under the native lock.
        // The engine writes all six slots (established by probing with
        // sentinel-filled buffers).
        unsafe {
            ffi::swe_cotrans_sp(
                input.as_ptr() as *mut c_double,
                output.as_mut_ptr(),
                eps as c_double,
            );
        }
        Ok((
            [output[0], output[1], output[2]],
            [output[3], output[4], output[5]],
        ))
    })
}

/// Normalize degrees into [0, 360).
///
/// Infallible, including for non-finite inputs (which propagate); a
/// poisoned lock degrades to the unlocked pure call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::degnorm;
///
/// assert_eq!(degnorm(-10.0), 350.0);
/// assert_eq!(degnorm(360.0), 0.0);
/// ```
pub fn degnorm(x: f64) -> f64 {
    // No shared native state is read or written; still serialized for
    // uniformity with the pointer-taking helpers above. A poisoned lock
    // degrades to the native call without the lock rather than failing
    // a pure computation.
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_degnorm(x as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_degnorm(x as c_double) }
    })
}

/// Normalize radians into [0, 2π).
///
/// Same contract as [`degnorm`].
///
/// # Examples
///
/// ```
/// use std::f64::consts::PI;
/// use swisseph_bindings::radnorm;
///
/// assert!((radnorm(7.0) - (7.0 - 2.0 * PI)).abs() < 1e-15);
/// ```
pub fn radnorm(x: f64) -> f64 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_radnorm(x as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_radnorm(x as c_double) }
    })
}

/// Signed `p1 − p2` angular distance in degrees, folded into [0, 360).
///
/// Same contract as [`degnorm`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::difdegn;
///
/// assert_eq!(difdegn(10.0, 350.0), 20.0);
/// assert_eq!(difdegn(350.0, 10.0), 340.0);
/// ```
pub fn difdegn(p1: f64, p2: f64) -> f64 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_difdegn(p1 as c_double, p2 as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_difdegn(p1 as c_double, p2 as c_double) }
    })
}

/// Signed `p1 − p2` angular distance in degrees, folded to ±180.
///
/// Same contract as [`degnorm`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::difdeg2n;
///
/// assert_eq!(difdeg2n(10.0, 350.0), 20.0);
/// assert_eq!(difdeg2n(350.0, 10.0), -20.0);
/// ```
pub fn difdeg2n(p1: f64, p2: f64) -> f64 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_difdeg2n(p1 as c_double, p2 as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_difdeg2n(p1 as c_double, p2 as c_double) }
    })
}

/// Signed `p1 − p2` angular distance in radians, folded to ±π.
///
/// Same contract as [`degnorm`].
///
/// # Examples
///
/// ```
/// use std::f64::consts::PI;
/// use swisseph_bindings::difrad2n;
///
/// assert!((difrad2n(0.1, 6.2) - (0.1 - 6.2 + 2.0 * PI)).abs() < 1e-15);
/// ```
pub fn difrad2n(p1: f64, p2: f64) -> f64 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_difrad2n(p1 as c_double, p2 as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_difrad2n(p1 as c_double, p2 as c_double) }
    })
}

/// Wrap-aware midpoint of two degree values.
///
/// Unlike a plain average, the seam is crossed the short way: 350° and
/// 10° meet at 0°, not at 180°. Same purity contract as [`degnorm`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::deg_midp;
///
/// assert_eq!(deg_midp(350.0, 10.0), 0.0);
/// ```
pub fn deg_midp(x1: f64, x0: f64) -> f64 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_deg_midp(x1 as c_double, x0 as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_deg_midp(x1 as c_double, x0 as c_double) }
    })
}

/// Wrap-aware midpoint of two radian values.
///
/// Same contract as [`deg_midp`].
///
/// # Examples
///
/// ```
/// use std::f64::consts::PI;
/// use swisseph_bindings::rad_midp;
///
/// // Short way across the 2π seam, from the wrap contract itself.
/// let want = (0.1 + 6.2 - 2.0 * PI) / 2.0;
/// assert!((rad_midp(0.1, 6.2) - want).abs() < 1e-15);
/// ```
pub fn rad_midp(x1: f64, x0: f64) -> f64 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_rad_midp(x1 as c_double, x0 as c_double) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_rad_midp(x1 as c_double, x0 as c_double) }
    })
}

/// Convert a float to a 32-bit integer with rounding.
///
/// Halves round away from zero (2.5 → 3, −2.5 → −3, established by
/// probing the pinned build).
///
/// # Errors
///
/// Non-finite inputs and magnitudes at or above `2_147_483_647.5`
/// are rejected with [`ErrorKind::InvalidInput`](crate::ErrorKind)
/// before any native call: the native conversion performs no range
/// check, so an unchecked out-of-range input would be undefined
/// behavior on the C side.
///
/// Both signs require a representable positive rounding intermediate;
/// `i32::MIN` is therefore outside the native conversion's domain.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::d2l;
///
/// assert_eq!(d2l(2.5)?, 3);
/// assert_eq!(d2l(-0.5)?, -1);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn d2l(x: f64) -> Result<i32, Error> {
    if !x.is_finite() || (x.abs() + 0.5).trunc() > f64::from(i32::MAX) {
        return Err(Error::invalid_input(format!(
            "d2l: value out of the i32-convertible range ({x})"
        )));
    }
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved; the domain
        // above keeps the native conversion in-range.
        Ok(unsafe { ffi::swe_d2l(x as c_double) as c_int })
    })
}

/// Normalize a centisecond angle into [0, 129600000).
///
/// One centisecond is 1/100 of an arcsecond (1/360000 of a degree), so
/// 129600000 centiseconds make a full turn. This is the integer-domain
/// counterpart of [`degnorm`]: `-360000` (−1°) reads back as `129240000`
/// (359°), and `129600000` (360°) reads back as `0`.
///
/// Infallible, including for any `i32` input; a poisoned lock degrades to
/// the unlocked pure call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::csnorm;
///
/// assert_eq!(csnorm(-360000), 129240000);
/// assert_eq!(csnorm(129600000), 0);
/// ```
pub fn csnorm(p: i32) -> i32 {
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved.
        Ok(unsafe { ffi::swe_csnorm(p) })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { ffi::swe_csnorm(p) }
    })
}

/// Signed `p1 − p2` centisecond distance, folded into [0, 129600000).
///
/// The integer-domain counterpart of [`difdegn`]: the result is always
/// non-negative, wrapping across the 360° seam.
///
/// Reads no shared state and no ephemeris files. Pairs whose subtraction
/// would overflow `i32` are rejected: the native subtraction performs no
/// range check, so an unchecked overflowing pair would be undefined
/// behavior on the C side (established by UBSan probing with
/// `p1 = i32::MAX, p2 = i32::MIN`).
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`](crate::ErrorKind) when
/// `p1.checked_sub(p2)` overflows. All astronomically meaningful angle
/// pairs (differences within a few turns) are unaffected; only pairs
/// whose raw difference cannot be represented as `i32` are rejected.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::difcsn;
///
/// assert_eq!(difcsn(720000, 360000)?, 360000);
/// assert_eq!(difcsn(360000, 720000)?, 129240000);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn difcsn(p1: i32, p2: i32) -> Result<i32, Error> {
    if p1.checked_sub(p2).is_none() {
        return Err(Error::invalid_input(format!(
            "difcsn: p1 - p2 overflows i32 (p1={p1}, p2={p2})"
        )));
    }
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved. The checked_sub
        // above keeps the native signed subtraction in-range.
        Ok(unsafe { ffi::swe_difcsn(p1, p2) })
    })
}

/// Signed `p1 − p2` centisecond distance, folded to ±64800000 (±180°).
///
/// The integer-domain counterpart of [`difdeg2n`]: the shorter arc wins,
/// and an exact half-turn reports −64800000 (established by probing the
/// pinned build).
///
/// Reads no shared state and no ephemeris files. Pairs whose subtraction would
/// overflow `i32` are rejected before any native call.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`](crate::ErrorKind) when
/// `p1.checked_sub(p2)` overflows.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::difcs2n;
///
/// assert_eq!(difcs2n(360000, 720000)?, -360000);
/// assert_eq!(difcs2n(64800000, 0)?, -64800000);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn difcs2n(p1: i32, p2: i32) -> Result<i32, Error> {
    if p1.checked_sub(p2).is_none() {
        return Err(Error::invalid_input(format!(
            "difcs2n: p1 - p2 overflows i32 (p1={p1}, p2={p2})"
        )));
    }
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved. The checked_sub
        // above keeps the native signed subtraction in-range.
        Ok(unsafe { ffi::swe_difcs2n(p1, p2) })
    })
}

/// Round a centisecond value to whole arcseconds.
///
/// The result is a multiple of 100 (1 arcsecond = 100 centiseconds).
/// Values just below a 30° sector boundary round down instead of
/// crossing into the next sector (established by probing the pinned
/// build: `10799950` reads back as `10799900`, not `10800000`).
///
/// Reads no shared state and no ephemeris files. Near the `i32` extremes:
/// the native rounding step adds/subtracts, so inputs within one
/// arcsecond (100 centiseconds) of `i32::MIN`/`i32::MAX` are rejected —
/// `swe_csroundsec(i32::MAX)` overflows natively (established by UBSan
/// probing). All astronomically meaningful angles (within a few turns,
/// i.e. within ±129600000 per turn) are unaffected.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`](crate::ErrorKind) when `x` lies
/// within 100 of either `i32` extreme.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::csroundsec;
///
/// assert_eq!(csroundsec(150)?, 200);
/// assert_eq!(csroundsec(149)?, 100);
/// assert_eq!(csroundsec(-150)?, -100);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn csroundsec(x: i32) -> Result<i32, Error> {
    if !(i32::MIN + 100..=i32::MAX - 100).contains(&x) {
        return Err(Error::invalid_input(format!(
            "csroundsec: value {x} is within 100 of the i32 extreme; native rounding would overflow"
        )));
    }
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved. The 100-margin
        // above keeps the native rounding addition/subtraction in-range.
        Ok(unsafe { ffi::swe_csroundsec(x) })
    })
}

/// Format centiseconds as an `HH<sep>MM<sep>SS` time string.
///
/// `cs` is a time in centiseconds (1 hour = 360000, 1 minute = 6000,
/// 1 second = 100); `sep` is the field separator byte (typically
/// `b':'`). When `suppress_zero` is set, a zero seconds field is
/// dropped (`"12:34"` instead of `"12:34:00"`). A rounding carry out of
/// `23:59:59.5x` wraps back to `"00:00:00"` (established by probing the
/// pinned build).
///
/// Pass non-negative values already wrapped into one day: negative
/// inputs format engine-defined (probed: `−360000` renders as
/// `"00:+':+'"` here), so callers normalize first (for example with
/// [`csnorm`]) instead of relying on the rendering.
///
/// # Errors
///
/// A NUL separator is rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call:
/// it would silently truncate the output. The absolute input plus the
/// native rounding addition of 50 must fit `i32`.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::cs2timestr;
///
/// assert_eq!(cs2timestr(0, b':', false)?, "00:00:00");
/// assert_eq!(cs2timestr(4526050, b':', false)?, "12:34:21");
/// assert_eq!(cs2timestr(4524000, b':', true)?, "12:34");
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn cs2timestr(cs: i32, sep: u8, suppress_zero: bool) -> Result<String, Error> {
    if sep == 0 {
        return Err(Error::invalid_input(
            "cs2timestr: separator must not be NUL (it would truncate the output)",
        ));
    }
    if cs.checked_abs().and_then(|v| v.checked_add(50)).is_none() {
        return Err(Error::invalid_input(
            "cs2timestr: absolute value plus rounding addition must fit i32",
        ));
    }
    with_native_access(|| {
        let mut buffer = [0 as c_char; FORMAT_BUF_LEN];
        // SAFETY: `buffer` owns FORMAT_BUF_LEN writable bytes, ample
        // for the 8/5-byte outputs (established by probing with
        // sentinel-filled buffers); it outlives this call under the
        // native lock and is read bounded before the lock is released.
        // Absolute value and the rounding addition are in-range above.
        unsafe {
            ffi::swe_cs2timestr(
                cs,
                sep as c_int,
                c_int::from(suppress_zero),
                buffer.as_mut_ptr(),
            );
            Ok(read_native_text(buffer.as_ptr(), FORMAT_BUF_LEN))
        }
    })
}

/// Format a centisecond angle with a direction character.
///
/// `plus` labels non-negative values (zero included) and `minus` labels
/// negative ones, whose absolute value is rendered: `360000` with
/// `b'N'`/`b'S'` gives `"1N00"`, `−360000` gives `"1S00"`. Whole minutes
/// render as `{deg}{dir}{min:02d}`; nonzero rounded seconds append
/// `'{sec:02d}` (probed: `12345678` with `b'E'`/`b'W'` gives
/// `"34E17'37"`).
///
/// # Errors
///
/// NUL direction characters are rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call.
/// The absolute input plus the native rounding addition of 50 must fit
/// `i32`; this excludes `i32::MIN` and magnitudes above `i32::MAX - 50`.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::cs2lonlatstr;
///
/// assert_eq!(cs2lonlatstr(360000, b'N', b'S')?, "1N00");
/// assert_eq!(cs2lonlatstr(-360000, b'N', b'S')?, "1S00");
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn cs2lonlatstr(cs: i32, plus: u8, minus: u8) -> Result<String, Error> {
    if plus == 0 || minus == 0 {
        return Err(Error::invalid_input(
            "cs2lonlatstr: direction characters must not be NUL (they would truncate the output)",
        ));
    }
    if cs.checked_abs().and_then(|v| v.checked_add(50)).is_none() {
        return Err(Error::invalid_input(
            "cs2lonlatstr: absolute value plus rounding addition must fit i32",
        ));
    }
    with_native_access(|| {
        let mut buffer = [0 as c_char; FORMAT_BUF_LEN];
        // SAFETY: as in `cs2timestr`; the longest probed output is
        // 8 bytes, far inside the owned buffer. Absolute value and the
        // rounding addition are representable after the guard above.
        unsafe {
            ffi::swe_cs2lonlatstr(cs, plus as c_char, minus as c_char, buffer.as_mut_ptr());
            Ok(read_native_text(buffer.as_ptr(), FORMAT_BUF_LEN))
        }
    })
}

/// Format a centisecond angle as whole degrees, minutes and arcseconds.
///
/// The layout is `{deg}°{min:02d}'{sec:02d}` with a UTF-8 degree sign
/// (U+00B0) and the seconds truncated to whole arcseconds — no
/// fractional part, no trailing quote (established by probing the pinned
/// build: `3723456` gives `"10°20'34"`). Negative values render the sign
/// on the degrees (`−360000` gives `"-1°00'00"`).
///
/// # Errors
///
/// `i32::MIN` is rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call:
/// its absolute value overflows natively. `Result` otherwise covers lock
/// poisoning only.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::cs2degstr;
///
/// assert_eq!(cs2degstr(360000)?, " 1°00'00");
/// assert_eq!(cs2degstr(-360000)?, "-1°00'00");
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn cs2degstr(cs: i32) -> Result<String, Error> {
    if cs == i32::MIN {
        return Err(Error::invalid_input(
            "cs2degstr: i32::MIN cannot be formatted (its absolute value overflows)",
        ));
    }
    with_native_access(|| {
        let mut buffer = [0 as c_char; FORMAT_BUF_LEN];
        // SAFETY: as in `cs2timestr`; the longest probed output is
        // 9 bytes, far inside the owned buffer.
        unsafe {
            ffi::swe_cs2degstr(cs, buffer.as_mut_ptr());
            Ok(read_native_text(buffer.as_ptr(), FORMAT_BUF_LEN))
        }
    })
}

/// Degree/minutes/seconds decomposition of an angle.
///
/// Returned by [`split_deg`]: `deg` holds whole degrees (within the
/// zodiac sign or nakshatra when those flags are used, otherwise the
/// total absolute degrees), `min` and `sec` hold whole arcminutes and
/// arcseconds (0-59), `secfr` holds the fractional arcseconds — or, when
/// a rounding flag is active, the rounded whole seconds as a float — and
/// `sign` holds `+1`/`−1` for an ordinary split, the 0-11 zodiac segment
/// with [`SPLIT_DEG_ZODIACAL`](crate::SPLIT_DEG_ZODIACAL), or the
/// nakshatra index with
/// [`SPLIT_DEG_NAKSHATRA`](crate::SPLIT_DEG_NAKSHATRA).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitDeg {
    /// Whole degrees (within the segment when a segment flag is used).
    pub deg: i32,
    /// Whole arcminutes (0-59).
    pub min: i32,
    /// Whole arcseconds (0-59).
    pub sec: i32,
    /// Fractional arcseconds, or the rounded seconds when rounding.
    pub secfr: f64,
    /// Sign (+1/−1) or segment index, per the flags used.
    pub sign: i32,
}

/// Split a degree value into degrees, minutes, seconds and sign/segment.
///
/// `roundflag` combines the [`SPLIT_DEG_*`](crate::SPLIT_DEG_ROUND_SEC)
/// bits: round to seconds/minutes/degrees, reduce into 30° zodiac
/// segments ([`SPLIT_DEG_ZODIACAL`](crate::SPLIT_DEG_ZODIACAL)) or
/// nakshatra segments
/// ([`SPLIT_DEG_NAKSHATRA`](crate::SPLIT_DEG_NAKSHATRA)), and guard the
/// rounding against crossing a sign
/// ([`SPLIT_DEG_KEEP_SIGN`](crate::SPLIT_DEG_KEEP_SIGN)) or degree
/// ([`SPLIT_DEG_KEEP_DEG`](crate::SPLIT_DEG_KEEP_DEG)) boundary. Unknown
/// bits pass through engine-defined; any integer is accepted, like the
/// method argument of [`crate::nod_aps`].
///
/// Negative inputs split their absolute value with `sign = −1`, except
/// in the zodiacal form, which splits the absolute value with the zodiac
/// segment index (probed: `−30.5` with `SPLIT_DEG_ZODIACAL` gives
/// `(0, 30, 0, 0.0, 1)`); the nakshatra form likewise falls back to the
/// signed split for negative inputs. A full turn reports segment `0`
/// (probed: `360.0` with `SPLIT_DEG_ZODIACAL` gives sign `0`).
///
/// # Errors
///
/// Non-finite inputs and magnitudes beyond the `i32`-representable degree
/// range are rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call:
/// the native decomposition performs no range check, so an unchecked
/// out-of-range magnitude (for example `1e100`, established by UBSan
/// probing) would be undefined behavior in the floating-to-integer
/// conversion. The admissible domain is
/// `-2147483647.0..=2147483647.0`: the whole-degree output is `i32`,
/// and the negative extreme is excluded because the engine splits the
/// absolute value. All ordinary angles (including multi-turn values) are
/// unaffected.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{SPLIT_DEG_ZODIACAL, split_deg};
///
/// let plain = split_deg(45.5, 0)?;
/// assert_eq!((plain.deg, plain.min, plain.sec, plain.sign), (45, 30, 0, 1));
/// let zodiac = split_deg(45.5, SPLIT_DEG_ZODIACAL)?;
/// assert_eq!((zodiac.deg, zodiac.min, zodiac.sign), (15, 30, 1));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn split_deg(degree: f64, roundflag: i32) -> Result<SplitDeg, Error> {
    if !degree.is_finite() || degree < -2147483647.0 || degree > 2147483647.0 {
        return Err(Error::invalid_input(format!(
            "split_deg: degree must be finite and within +-2147483647.0 ({degree})"
        )));
    }
    with_native_access(|| {
        let mut deg: i32 = 0;
        let mut min: i32 = 0;
        let mut sec: i32 = 0;
        let mut secfr: c_double = 0.0;
        let mut sign: i32 = 0;
        // SAFETY: each out variable owns exactly one writable slot of
        // the declared type and outlives this call under the native
        // lock. The engine writes all five slots. The domain check above
        // keeps every native floating-to-integer conversion in-range.
        unsafe {
            ffi::swe_split_deg(
                degree as c_double,
                roundflag,
                &mut deg,
                &mut min,
                &mut sec,
                &mut secfr,
                &mut sign,
            );
        }
        Ok(SplitDeg {
            deg,
            min,
            sec,
            secfr,
            sign,
        })
    })
}
