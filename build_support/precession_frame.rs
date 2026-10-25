//! Share one dimensionless precession matrix between position and velocity.
//!
//! Only the adjacent projections in one native operation share preparation.
//! Their epoch is the same Julian Day in native ephemeris time (TT), flags and
//! direction are identical, and their vectors do not overlap the local frame.
//! Scalar entry points, model selection and all projection expressions remain
//! native. No preparation survives the owning operation.

use super::c_source::{body_after, fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/precession_frame.h";
const NATIVE_CATALOG: u64 = 0x811a_3282_bb39_3790;
const NATIVE_LIBRARY: u64 = 0xe642_4aa4_7d15_66b1;
const PRECEDING_CATALOG: u64 = 0xc9f5_d4b0_0ad9_a6e8;
const PRECEDING_LIBRARY: u64 = 0x7b63_9529_e9dc_b654;
const GENERATED_CATALOG: u64 = 0x2342_d9f1_dad7_344f;
const GENERATED_LIBRARY: u64 = 0x947c_db84_c360_477f;
const HELPER_FINGERPRINT: u64 = 0xa82f_97ac_6b4f_14ca;

/// Source and private scalar-ABI header declared to Cargo and native identity.
pub const INPUTS: [&str; 2] = ["build_support/precession_frame.rs", HELPER];

// The newline excludes the otherwise identical forward declaration. The body
// reader must select one definition before locating the pair inside that body.
const APP_SIGNATURE: &str = r#"static int app_pos_etc_plan(int ipli, int iplmoon, int32 iflag, char *serr)
"#;
const ORIGINAL_PAIR: &str = r#"    swi_precess(xx, pdp->teval, iflag, J2000_TO_J);
    if (iflag & SEFLG_SPEED)
      swi_precess_speed(xx, pdp->teval, iflag, J2000_TO_J);"#;
const SHARED_PAIR: &str = r#"    /* One stack-local frame prepares this operation's matrix for position
     * and velocity. It is never retained or shared with another request. */
    struct swe_precession_frame frame = {{0}, 0};
    swe_precess_with_frame(xx, pdp->teval, iflag, J2000_TO_J, &frame);
    if (iflag & SEFLG_SPEED)
      swe_precess_speed_with_frame(xx, pdp->teval, iflag, J2000_TO_J, &frame);"#;

struct Edit {
    before: &'static str,
    after: &'static str,
    count: usize,
}

const LIBRARY_EDITS: &[Edit] = &[
    Edit {
        before: r#"#include "swephlib.h""#,
        after: r#"#include "swephlib.h"
#include "precession_frame.h""#,
        count: 1,
    },
    Edit {
        before: r#"static int precess_3(double *R, double J, int direction, int iflag, int prec_meth)"#,
        after: r#"static int precess_3(double *R, double J, int direction, int iflag, int prec_meth, struct swe_precession_frame *frame)"#,
        count: 1,
    },
    Edit {
        before: r#"  double x[3], pmat[9];"#,
        after: r#"  double x[3], local_matrix[9];
  double *pmat = frame != NULL ? frame->matrix : local_matrix;"#,
        count: 1,
    },
    Edit {
        before: r#"  if (prec_meth == SEMOD_PREC_OWEN_1990)
    owen_pre_matrix(J, pmat, iflag);
  else
    pre_pmat(J, pmat);"#,
        after: r#"  /* Both vectors in the one owning operation use these same inputs.
   * Only matrix preparation is shared; each projection keeps its native loop. */
  if (frame == NULL || !frame->prepared) {
    if (prec_meth == SEMOD_PREC_OWEN_1990)
      owen_pre_matrix(J, pmat, iflag);
    else
      pre_pmat(J, pmat);
    if (frame != NULL)
      frame->prepared = 1;
  }"#,
        count: 1,
    },
    Edit {
        before: r#"int swi_precess(double *R, double J, int32 iflag, int direction )"#,
        after: r#"static int swe_precess_dispatch(double *R, double J, int32 iflag, int direction, struct swe_precession_frame *frame)"#,
        count: 1,
    },
    Edit {
        before: r#"return precess_3(R, J, direction, iflag, SEMOD_PREC_OWEN_1990);"#,
        after: r#"return precess_3(R, J, direction, iflag, SEMOD_PREC_OWEN_1990, frame);"#,
        count: 2,
    },
    Edit {
        before: r#"return precess_3(R, J, direction, iflag, SEMOD_PREC_VONDRAK_2011);"#,
        after: r#"return precess_3(R, J, direction, iflag, SEMOD_PREC_VONDRAK_2011, frame);"#,
        count: 1,
    },
    Edit {
        before: r#"
/* Nutation in longitude and obliquity
"#,
        after: r#"
/* The public scalar entry never retains or shares a preparation. */
int swi_precess(double *R, double J, int32 iflag, int direction)
{
  return swe_precess_dispatch(R, J, iflag, direction, NULL);
}

/* Private companion: its owner proves unchanged day/flags/direction for the
 * adjacent position and velocity projections; no caller callback intervenes. */
void swe_precess_with_frame(double *R, double J, int32 iflag, int direction,
                           struct swe_precession_frame *frame)
{
  swe_precess_dispatch(R, J, iflag, direction, frame);
}

/* Nutation in longitude and obliquity
"#,
        count: 1,
    },
];

const CATALOG_EDITS: &[Edit] = &[
    Edit {
        before: r#"#include "swephlib.h""#,
        after: r#"#include "swephlib.h"
#include "precession_frame.h""#,
        count: 1,
    },
    Edit {
        before: r#"void swi_precess_speed(double *xx, double t, int32 iflag, int direction) "#,
        after: r#"static void swe_precess_speed_with_frame(double *xx, double t, int32 iflag, int direction, struct swe_precession_frame *frame)"#,
        count: 1,
    },
    Edit {
        before: r#"  swi_precess(xx+3, t, iflag, direction); "#,
        after: r#"  if (frame != NULL)
    swe_precess_with_frame(xx+3, t, iflag, direction, frame);
  else
    swi_precess(xx+3, t, iflag, direction); "#,
        count: 1,
    },
    Edit {
        before: r#"#include "precession_frame.h""#,
        after: r#"#include "precession_frame.h"
static void swe_precess_speed_with_frame(double *xx, double t, int32 iflag,
                                         int direction, struct swe_precession_frame *frame);
"#,
        count: 1,
    },
    Edit {
        before: r#"
/* multiplies cartesian equatorial coordinates with previously
"#,
        after: r#"
/* Retain the existing scalar public operation and its original preparation order. */
void swi_precess_speed(double *xx, double t, int32 iflag, int direction)
{
  swe_precess_speed_with_frame(xx, t, iflag, direction, NULL);
}

/* multiplies cartesian equatorial coordinates with previously
"#,
        count: 1,
    },
];

/// Apply after the filename overlay to the two generated translation units.
///
/// Panics on any unreviewed native, preceding, helper or generated bytes, or an
/// I/O failure. Only generated files are written; vendor sources stay intact.
pub fn apply(native: &Path, catalog: &Path, library: &Path) {
    let native_catalog =
        fs::read_to_string(native.join("sweph.c")).expect("read native precession caller");
    let native_library =
        fs::read_to_string(native.join("swephlib.c")).expect("read native precession selector");
    let preceding_catalog = fs::read_to_string(catalog).expect("read preceding catalog overlays");
    let preceding_library = fs::read_to_string(library).expect("read preceding library overlays");
    let helper = fs::read_to_string(HELPER).expect("read local precession header");
    let (output_catalog, output_library) = transformed(
        &native_catalog,
        &native_library,
        &preceding_catalog,
        &preceding_library,
        &helper,
    )
    .expect("reviewed local precession preparation");
    fs::write(catalog, output_catalog).expect("write local precession caller");
    fs::write(library, output_library).expect("write local precession selector");
}

fn require_identity(source: &str, expected: u64) -> Result<(), String> {
    if fingerprint(source.as_bytes()) == expected {
        Ok(())
    } else {
        Err("unreviewed complete local precession input".into())
    }
}

fn edited(source: &str, edits: &[Edit], reverse: bool) -> Result<String, String> {
    let mut output = source.to_owned();
    let indices: Vec<_> = if reverse {
        (0..edits.len()).rev().collect()
    } else {
        (0..edits.len()).collect()
    };
    for index in indices {
        let edit = &edits[index];
        let (before, after) = if reverse {
            (edit.after, edit.before)
        } else {
            (edit.before, edit.after)
        };
        if output.matches(before).count() != edit.count {
            return Err("local precession fragment multiplicity differs".into());
        }
        output = output.replacen(before, after, edit.count);
    }
    Ok(output)
}

fn edit_caller(source: &str, before: &str, after: &str) -> Result<String, String> {
    let body = body_after(source, APP_SIGNATURE)?;
    let offset = unique_offset(source, body)? + unique_offset(body, before)?;
    let mut output = source.to_owned();
    output.replace_range(offset..offset + before.len(), after);
    Ok(output)
}

/// Remove only this overlay from the generated library unit.
///
/// The complete output and recovered preceding identities are mandatory. This
/// lets preceding fixtures retain their own original guards and inverse proofs.
/// Unreviewed or already-restored text is rejected; no C operation is run.
pub fn restore_lib(source: &str) -> Result<String, String> {
    require_identity(source, GENERATED_LIBRARY)?;
    let restored = edited(source, LIBRARY_EDITS, true)?;
    require_identity(&restored, PRECEDING_LIBRARY)?;
    Ok(restored)
}

/// Remove only this overlay from the generated caller unit, or reject its bytes.
pub(super) fn restore_catalog(source: &str) -> Result<String, String> {
    require_identity(source, GENERATED_CATALOG)?;
    let restored = edited(source, CATALOG_EDITS, true)?;
    let restored = edit_caller(&restored, SHARED_PAIR, ORIGINAL_PAIR)?;
    require_identity(&restored, PRECEDING_CATALOG)?;
    Ok(restored)
}

/// Emit the one admitted composition; errors never yield a partial transform.
pub(super) fn transformed(
    native_catalog: &str,
    native_library: &str,
    preceding_catalog: &str,
    preceding_library: &str,
    helper: &str,
) -> Result<(String, String), String> {
    for (source, identity) in [
        (native_catalog, NATIVE_CATALOG),
        (native_library, NATIVE_LIBRARY),
        (preceding_catalog, PRECEDING_CATALOG),
        (preceding_library, PRECEDING_LIBRARY),
        (helper, HELPER_FINGERPRINT),
    ] {
        require_identity(source, identity)?;
    }
    let caller = edit_caller(preceding_catalog, ORIGINAL_PAIR, SHARED_PAIR)?;
    let catalog = edited(&caller, CATALOG_EDITS, false)?;
    let library = edited(preceding_library, LIBRARY_EDITS, false)?;
    if restore_catalog(&catalog)? != preceding_catalog
        || restore_lib(&library)? != preceding_library
    {
        return Err("local precession composition is not byte-reversible".into());
    }
    Ok((catalog, library))
}
