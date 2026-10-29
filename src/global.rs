//! Legacy process-global operations with their original 1.x signatures.
//!
//! These are direct re-exports, not argument adapters. In particular,
//! [`nod_aps`] and [`nod_aps_ut`] retain `(jd, body, flags, method)` and observer
//! operations retain the original scalar arguments. The primary Session API
//! has its own contracted ordering; it must not change these entry points.
//!
//! Each function serializes its complete native sequence with the existing
//! process-wide lock. A setter followed by a separate calculation is **not**
//! atomic and may observe another caller's configuration. Use [`crate::Session`]
//! for an owned configuration applied together with its dependent calculation.
//! These functions neither create isolated native engines nor authenticate
//! data provenance from returned source-flag bits.
//!
//! Units, time scales, buffers, native flags/status and errors are documented
//! on the individual operations and remain unchanged by this namespace.

pub use crate::config::{
    close, get_ayanamsa, get_ayanamsa_ex, get_ayanamsa_ex_ut, get_ayanamsa_ut,
    get_current_file_data, get_tid_acc, set_delta_t_userdef, set_ephe_path, set_sid_mode,
    set_tid_acc, set_topo,
};
pub use crate::events::{
    heliacal_pheno_ut, heliacal_ut, helio_cross, helio_cross_ut, lun_eclipse_how, lun_eclipse_when,
    lun_eclipse_when_loc, lun_occult_when_glob, lun_occult_when_loc, lun_occult_where, mooncross,
    mooncross_node, mooncross_node_ut, mooncross_ut, sol_eclipse_how, sol_eclipse_when_glob,
    sol_eclipse_when_loc, sol_eclipse_where, solcross, solcross_ut, vis_limit_mag,
};
pub use crate::houses::{
    gauquelin_sector, house_pos, houses, houses_armc, houses_armc_ex2, houses_ex, houses_ex2,
    houses_gauquelin,
};
pub use crate::observer::{
    azalt, azalt_rev, get_orbital_elements, nod_aps, nod_aps_ut, orbit_max_min_true_distance,
    pheno, pheno_ut, refrac, refrac_extended, rise_trans, rise_trans_true_hor, set_lapse_rate,
};
pub use crate::positions::{calc, calc_pctr, calc_ut, get_planet_name};
pub use crate::stars::{fixstar, fixstar_mag, fixstar_ut, fixstar2, fixstar2_mag, fixstar2_ut};
pub use crate::time::{
    deltat, deltat_ex, jdet_to_utc, jdut1_to_utc, lat_to_lmt, lmt_to_lat, sidtime, sidtime0,
    time_equ, utc_to_jd,
};
