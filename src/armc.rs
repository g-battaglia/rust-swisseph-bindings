//! Native ARMC buffers shared by fixed SDK and full primary transports.
use crate::{error::Error, ffi};
use std::ffi::{c_char, c_int};

/// Validate finite geometry and exact native buffer capacity before C writes.
fn check<const N: usize>(
    armc: f64,
    latitude: f64,
    obliquity: f64,
    system: u8,
) -> Result<(), Error> {
    let required = if system.eq_ignore_ascii_case(&b'G') {
        37
    } else {
        13
    };
    if N != required || !system.is_ascii_alphabetic() {
        return Err(Error::invalid_input(
            "houses_armc: invalid system/buffer layout",
        ));
    }
    if !armc.is_finite() || !latitude.is_finite() || !obliquity.is_finite() {
        return Err(Error::invalid_input(
            "houses_armc: armc, latitude and eps must be finite",
        ));
    }
    Ok(())
}

/// Execute one ARMC calculation under the caller's native lock.
/// Degrees in/out; N is 13 for houses or 37 for Gauquelin, including reserved slot zero.
pub(crate) fn values<const N: usize>(
    armc: f64,
    latitude: f64,
    obliquity: f64,
    system: u8,
) -> Result<([f64; N], [f64; 10], i32), Error> {
    check::<N>(armc, latitude, obliquity, system)?;
    let mut cusps = [0.0; N];
    let mut angles = [0.0; 10];
    // SAFETY: capacity was checked against the native system (13 or 37 cusps,
    // ten angles). Both initialized buffers are exclusively owned for the call,
    // remain live and unaliased, and the caller holds the native access lock.
    let status = unsafe {
        ffi::swe_houses_armc(
            armc,
            latitude,
            obliquity,
            c_int::from(system),
            cusps.as_mut_ptr(),
            angles.as_mut_ptr(),
        )
    };
    if status < 0 {
        return Err(Error::native(format!("houses_armc: native computation failed (armc={armc}, lat={latitude}, eps={obliquity}, hsys={})", system as char))
            .with_native_channels("houses_armc", Some(status), None, None));
    }
    Ok((cusps, angles, status))
}

/// Original complete native ARMC rate buffers with status and diagnostic.
pub(crate) struct Rates<const N: usize> {
    pub cusps: [f64; N],
    pub angles: [f64; 10],
    pub cusp_speeds: [f64; N],
    pub angle_speeds: [f64; 10],
    pub status: i32,
    pub diagnostic: String,
}

/// Execute the native speed operation with an explicit solar declination.
/// Angles are degrees and rates degrees/day; all buffers retain their native widths.
pub(crate) fn rates<const N: usize>(
    armc: f64,
    latitude: f64,
    obliquity: f64,
    system: u8,
    sun_declination: f64,
) -> Result<Rates<N>, Error> {
    check::<N>(armc, latitude, obliquity, system)?;
    if !sun_declination.is_finite() {
        return Err(Error::invalid_input(
            "houses_armc_ex2: sun_declination must be finite",
        ));
    }
    let mut output = Rates {
        cusps: [0.0; N],
        angles: [0.0; 10],
        cusp_speeds: [0.0; N],
        angle_speeds: [0.0; 10],
        status: 0,
        diagnostic: String::new(),
    };
    output.angles[9] = sun_declination;
    let mut diagnostic = [0 as c_char; ffi::SERR_LEN];
    // SAFETY: capacities match the checked system, ten angles/rates and the
    // native diagnostic width. All buffers are initialized, owned and distinct,
    // and remain live and unaliased while the caller holds native access.
    output.status = unsafe {
        ffi::swe_houses_armc_ex2(
            armc,
            latitude,
            obliquity,
            c_int::from(system),
            output.cusps.as_mut_ptr(),
            output.angles.as_mut_ptr(),
            output.cusp_speeds.as_mut_ptr(),
            output.angle_speeds.as_mut_ptr(),
            diagnostic.as_mut_ptr(),
        )
    };
    // SAFETY: read is bounded by the initialized owned diagnostic capacity.
    output.diagnostic = unsafe { ffi::read_native_text(diagnostic.as_ptr(), ffi::SERR_LEN) };
    if output.status < 0 {
        return Err(Error::native(format!(
            "houses_armc_ex2: native computation failed (armc={armc}, hsys={}): {}",
            system as char, output.diagnostic
        ))
        .with_native_channels(
            "houses_armc_ex2",
            Some(output.status),
            None,
            Some(&output.diagnostic),
        ));
    }
    Ok(output)
}
