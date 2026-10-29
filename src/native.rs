//! Native-specific extensions and original 1.x pure-function signatures.
//!
//! These direct re-exports preserve their original arguments and native
//! outputs. The raw pure utilities are retained here independently of the
//! primary API's final typed results. The `*_full` functions retain complete
//! native buffers; they must not be replaced by padded or reconstructed data.
//!
//! Configuration/catalogue helpers in this module can still read or change
//! process-global native state under the existing lock. A native setter and a
//! later computation are not atomic. [`crate::Session`] is the primary route
//! for calculations using an owned configuration.
//!
//! [`require_source_flags`] compares reported flag bits only; it is not proof
//! of the physical data provider. File/library metadata helpers report the
//! pinned native engine's own information, not a cross-engine capability API.
//! Individual items document their units, domains, error conditions and limits.

pub use crate::angles::{
    SplitDeg, cotrans, cotrans_sp, cs2degstr, cs2lonlatstr, cs2timestr, csnorm, csroundsec, d2l,
    deg_midp, degnorm, difcs2n, difcsn, difdeg2n, difdegn, difrad2n, rad_midp, radnorm, split_deg,
    split_deg_2_10_03,
};
pub use crate::config::{
    CurrentFileData, MAX_EPHE_PATH_LEN, MAX_JPL_FILE_LEN, close_preserving_path, ensure_ephe_path,
    get_ayanamsa_name, set_jpl_file,
};
pub use crate::events::{
    HeliacalPhenomenaFull, OccultSearchOptions, OccultationGeometryFull, SolarEclipseGeometryFull,
    VisibilityFull, heliacal_pheno_ut_full, lun_occult_when_glob_with_options,
    lun_occult_when_loc_with_options, lun_occult_where_full, sol_eclipse_where_full,
    vis_limit_mag_full,
};
pub use crate::houses::house_name;
pub use crate::observer::NodesApsides;
pub use crate::positions::require_source_flags;
pub use crate::stars::fixed_star_catalog;
pub use crate::time::{date_conversion, day_of_week, julday, revjul, utc_time_zone};
pub use crate::version::{library_path, version};

// Original SDK records and configured session, independent of primary shapes.
pub use crate::global::*;
pub use crate::legacy::*;
pub use crate::{global, native};

/// Create a closed primary session from explicit native SDK settings.
///
/// File paths and JPL selection remain engine-specific. All numerical settings
/// use owned defaults unless overridden; construction does not install C state.
///
/// # Errors
/// Invalid configuration and unavailable external lookup context propagate.
pub fn primary_session(
    builder: &crate::session::SessionBuilder,
) -> Result<crate::Session, crate::Error> {
    crate::primary::from_native_builder(builder)
}

/// Node/apse transport of the retained configured SDK session.
pub use crate::node_result::NodesApsides as SessionNodesApsides;
