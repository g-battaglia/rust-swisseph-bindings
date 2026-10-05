//! Safe visibility and heliacal APIs. Atmosphere/observer inputs are fresh
//! value arrays for every call; native default writes cannot alter a later case.
use super::protocol::{Record, Request};
use super::registry::Op;
use swisseph_bindings::*;

/// Dispatch arrays in pressure/temperature/humidity/range and native six-slot
/// observer order. Times are UT days, geography degrees and elevation metres.
pub fn call(op: Op, r: &Request) -> Result<Record, Error> {
    let atmosphere = [r.f(4), r.f(5), r.f(6), r.f(7)];
    let observer = [r.f(8), r.f(9), r.f(10), r.f(11), r.f(12), r.f(13)];
    match op {
        Op::HeliacalUt => heliacal_ut(
            r.f(0),
            r.f(1),
            r.f(2),
            r.f(3),
            atmosphere,
            observer,
            r.text(0),
            r.i(0),
            r.i(1),
        )
        .map(|v| Record {
            floats: v
                .times
                .into_iter()
                .chain([v.begin(), v.optimum(), v.end()])
                .collect(),
            texts: vec![v.diagnostic.into_bytes()],
            ..Record::default()
        }),
        Op::HeliacalPhenoUt => heliacal_pheno_ut(
            r.f(0),
            r.f(1),
            r.f(2),
            r.f(3),
            atmosphere,
            observer,
            r.text(0),
            r.i(0),
            r.i(1),
        )
        .map(|v| Record {
            floats: v
                .values
                .into_iter()
                .chain([
                    v.object_altitude_deg(),
                    v.apparent_altitude_deg(),
                    v.geocentric_altitude_deg(),
                    v.object_azimuth_deg(),
                    v.sun_altitude_deg(),
                    v.sun_azimuth_deg(),
                    v.topocentric_arcus_visionis_deg(),
                    v.arcus_visionis_deg(),
                    v.azimuth_difference_deg(),
                    v.longitude_difference_deg(),
                    v.extinction_coefficient(),
                    v.min_topocentric_arcus_visionis_deg(),
                    v.first_visible_jd(),
                    v.best_visible_jd(),
                    v.last_visible_jd(),
                    v.yallop_best_jd(),
                    v.moon_crescent_width_deg(),
                    v.yallop_q(),
                    v.yallop_criterion(),
                    v.parallax_deg(),
                    v.object_magnitude(),
                    v.object_rise_set_jd(),
                    v.sun_rise_set_jd(),
                    v.lag_days(),
                    v.visibility_duration_days(),
                    v.moon_crescent_length_deg(),
                    v.crescent_visibility_arc_deg(),
                    v.illumination_percent(),
                ])
                .collect(),
            texts: vec![v.diagnostic.into_bytes()],
            ..Record::default()
        }),
        Op::VisLimitMag => vis_limit_mag(
            r.f(0),
            r.f(1),
            r.f(2),
            r.f(3),
            atmosphere,
            observer,
            r.text(0),
            r.i(0),
        )
        .map(|v| match v {
            VisibilityOutcome::Visible(v) => Record {
                code: v.status,
                floats: v
                    .values
                    .into_iter()
                    .chain([
                        v.limiting_magnitude(),
                        v.object_altitude_deg(),
                        v.object_azimuth_deg(),
                        v.sun_altitude_deg(),
                        v.sun_azimuth_deg(),
                        v.object_magnitude(),
                    ])
                    .collect(),
                texts: vec![v.diagnostic.into_bytes()],
                ..Record::default()
            },
            VisibilityOutcome::BelowHorizon(v) => Record {
                code: -2,
                floats: v.values.to_vec(),
                texts: vec![v.diagnostic.into_bytes()],
                ..Record::default()
            },
        }),
        _ => unreachable!("validated heliacal operation"),
    }
}
