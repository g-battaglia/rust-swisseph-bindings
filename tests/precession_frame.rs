//! Source-only composition proofs for one operation-local precession frame.
#![allow(dead_code)]

#[path = "../build_support/c_source.rs"]
mod c_source;
#[path = "../build_support/precession_frame.rs"]
mod precession_frame;

const NATIVE_CATALOG: &str = include_str!("../swisseph/sweph.c");
const NATIVE_LIBRARY: &str = include_str!("../swisseph/swephlib.c");
const CATALOG: &str = include_str!(concat!(env!("OUT_DIR"), "/sweph.c"));
const LIBRARY: &str = include_str!(concat!(env!("OUT_DIR"), "/swephlib.c"));
const HELPER: &str = include_str!("../src/native/precession_frame.h");

#[test]
fn restores_complete_preceding_units_and_reproduces_the_exact_composition() {
    let catalog = precession_frame::restore_catalog(CATALOG).unwrap();
    let library = precession_frame::restore_lib(LIBRARY).unwrap();
    assert_eq!(
        c_source::fingerprint(catalog.as_bytes()),
        0xc9f5_d4b0_0ad9_a6e8
    );
    assert_eq!(
        c_source::fingerprint(library.as_bytes()),
        0x7b63_9529_e9dc_b654
    );
    let output =
        precession_frame::transformed(NATIVE_CATALOG, NATIVE_LIBRARY, &catalog, &library, HELPER)
            .unwrap();
    assert_eq!(output.0, CATALOG);
    assert_eq!(output.1, LIBRARY);
}

#[test]
fn rejects_changed_native_helper_preceding_or_final_bytes_and_wrong_order() {
    let catalog = precession_frame::restore_catalog(CATALOG).unwrap();
    let library = precession_frame::restore_lib(LIBRARY).unwrap();
    for changed in 0..5 {
        let mut inputs =
            [NATIVE_CATALOG, NATIVE_LIBRARY, &catalog, &library, HELPER].map(str::to_owned);
        inputs[changed].push('\n');
        assert!(
            precession_frame::transformed(
                &inputs[0], &inputs[1], &inputs[2], &inputs[3], &inputs[4],
            )
            .is_err()
        );
    }
    assert!(
        precession_frame::transformed(NATIVE_CATALOG, NATIVE_LIBRARY, CATALOG, LIBRARY, HELPER,)
            .is_err()
    );
    assert!(precession_frame::restore_catalog(&format!("{CATALOG}\n")).is_err());
    assert!(precession_frame::restore_lib(&format!("{LIBRARY}\n")).is_err());
    assert!(precession_frame::restore_lib(&library).is_err());
}

#[test]
fn confines_frame_to_the_owning_pair_and_keeps_the_entire_speed_body() {
    let prior = precession_frame::restore_catalog(CATALOG).unwrap();
    let signature = "static int app_pos_etc_plan(int ipli, int iplmoon, int32 iflag, char *serr)\n";
    let caller = c_source::body_after(CATALOG, signature).unwrap();
    assert_eq!(
        CATALOG
            .matches("struct swe_precession_frame frame =")
            .count(),
        1
    );
    assert!(caller.contains("swe_precess_with_frame(xx, pdp->teval, iflag, J2000_TO_J, &frame);"));
    assert!(
        caller.contains("swe_precess_speed_with_frame(xx, pdp->teval, iflag, J2000_TO_J, &frame);")
    );
    let old_body = c_source::body_after(
        &prior,
        "void swi_precess_speed(double *xx, double t, int32 iflag, int direction)",
    )
    .unwrap();
    let new_body = c_source::body_after(CATALOG, "static void swe_precess_speed_with_frame(double *xx, double t, int32 iflag, int direction, struct swe_precession_frame *frame)\n").unwrap();
    let rotation = "  if (frame != NULL)\n    swe_precess_with_frame(xx+3, t, iflag, direction, frame);\n  else\n    swi_precess(xx+3, t, iflag, direction); ";
    assert_eq!(new_body.matches(rotation).count(), 1);
    assert_eq!(
        new_body.replace(rotation, "  swi_precess(xx+3, t, iflag, direction); "),
        old_body
    );
    let wrapper = c_source::body_after(
        CATALOG,
        "void swi_precess_speed(double *xx, double t, int32 iflag, int direction)\n",
    )
    .unwrap();
    assert!(wrapper.contains("swe_precess_speed_with_frame(xx, t, iflag, direction, NULL);"));
}
