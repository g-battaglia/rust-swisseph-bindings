//! Public fixed-star and observer/physical calls. Every component is copied in
//! the declared public order; buffers, source/status flags and text are not
//! inferred from scientific relationships between separate calculations.
use super::protocol::{Record, Request};
use super::registry::Op;
use swisseph_bindings::*;

/// Pack only the public value/diagnostic pair shared by these native operations.
fn value_diagnostic(value: f64, diagnostic: String) -> Record {
    Record {
        floats: vec![value],
        texts: vec![diagnostic.into_bytes()],
        ..Record::default()
    }
}

/// Dispatch observer and catalog operations after exact argument validation.
/// Array inputs to heliacal/default-writing calls are handled separately.
pub fn call(op: Op, r: &Request) -> Result<Record, Error> {
    use Op::*;
    match op {
        Fixstar | FixstarUt | Fixstar2 | Fixstar2Ut => {
            let value = match op {
                Fixstar => fixstar(r.text(0), r.f(0), r.i(0)),
                FixstarUt => fixstar_ut(r.text(0), r.f(0), r.i(0)),
                Fixstar2 => fixstar2(r.text(0), r.f(0), r.i(0)),
                _ => fixstar2_ut(r.text(0), r.f(0), r.i(0)),
            }?;
            Ok(Record {
                code: value.returned_flags,
                floats: value
                    .values
                    .into_iter()
                    .chain([value.longitude(), value.latitude(), value.distance()])
                    .collect(),
                texts: vec![
                    value.resolved_name.into_bytes(),
                    value.diagnostic.into_bytes(),
                ],
                ..Record::default()
            })
        }
        FixstarMag | Fixstar2Mag => {
            let value = if op == FixstarMag {
                fixstar_mag(r.text(0))
            } else {
                fixstar2_mag(r.text(0))
            }?;
            Ok(Record {
                floats: vec![value.magnitude],
                texts: vec![
                    value.resolved_name.into_bytes(),
                    value.diagnostic.into_bytes(),
                ],
                ..Record::default()
            })
        }
        GauquelinSector => gauquelin_sector(
            r.f(0),
            r.i(0),
            r.optional_text(0),
            r.i(1),
            r.i(2),
            r.f(1),
            r.f(2),
            r.f(3),
            r.f(4),
            r.f(5),
        )
        .map(|v| value_diagnostic(v.value, v.diagnostic)),
        NodAps | NodApsUt => {
            let value = if op == NodAps {
                nod_aps(r.f(0), r.i(0), r.i(1), r.i(2))
            } else {
                nod_aps_ut(r.f(0), r.i(0), r.i(1), r.i(2))
            }?;
            Ok(Record {
                code: value.returned_flags,
                floats: value
                    .ascending
                    .into_iter()
                    .chain(value.descending)
                    .chain(value.perihelion)
                    .chain(value.aphelion)
                    .collect(),
                texts: vec![value.diagnostic.into_bytes()],
                ..Record::default()
            })
        }
        RiseTrans | RiseTransTrueHor => {
            let value = if op == RiseTrans {
                rise_trans(
                    r.f(0),
                    r.i(0),
                    r.optional_text(0),
                    r.i(1),
                    r.i(2),
                    r.f(1),
                    r.f(2),
                    r.f(3),
                    r.f(4),
                    r.f(5),
                )
            } else {
                rise_trans_true_hor(
                    r.f(0),
                    r.i(0),
                    r.optional_text(0),
                    r.i(1),
                    r.i(2),
                    r.f(1),
                    r.f(2),
                    r.f(3),
                    r.f(4),
                    r.f(5),
                    r.f(6),
                )
            }?;
            Ok(match value {
                RiseTransitOutcome::Event {
                    time_ut,
                    diagnostic,
                } => value_diagnostic(time_ut, diagnostic),
                RiseTransitOutcome::Circumpolar { diagnostic } => Record {
                    code: -2,
                    texts: vec![diagnostic.into_bytes()],
                    ..Record::default()
                },
            })
        }
        Azalt => azalt(
            r.f(0),
            r.i(0),
            r.f(1),
            r.f(2),
            r.f(3),
            r.f(4),
            r.f(5),
            r.f(6),
            r.f(7),
        )
        .map(|v| Record {
            floats: vec![v.azimuth, v.true_altitude, v.apparent_altitude],
            ..Record::default()
        }),
        AzaltRev => {
            azalt_rev(r.f(0), r.i(0), r.f(1), r.f(2), r.f(3), r.f(4), r.f(5)).map(|v| Record {
                floats: vec![v.longitude_or_ra, v.latitude_or_dec],
                ..Record::default()
            })
        }
        Refrac => refrac(r.f(0), r.f(1), r.f(2), r.i(0)).map(|v| Record {
            floats: vec![v],
            ..Record::default()
        }),
        RefracExtended => {
            refrac_extended(r.f(0), r.f(1), r.f(2), r.f(3), r.f(4), r.i(0)).map(|v| Record {
                floats: vec![
                    v.converted,
                    v.true_altitude,
                    v.apparent_altitude,
                    v.refraction,
                    v.dip,
                ],
                ..Record::default()
            })
        }
        Pheno | PhenoUt => {
            let value = if op == Pheno {
                pheno(r.f(0), r.i(0), r.i(1))
            } else {
                pheno_ut(r.f(0), r.i(0), r.i(1))
            }?;
            Ok(Record {
                code: value.returned_flags,
                floats: value
                    .values
                    .into_iter()
                    .chain([
                        value.phase_angle(),
                        value.illuminated_fraction(),
                        value.elongation(),
                        value.apparent_diameter(),
                        value.magnitude(),
                    ])
                    .collect(),
                texts: vec![value.diagnostic.into_bytes()],
                ..Record::default()
            })
        }
        GetOrbitalElements => get_orbital_elements(r.f(0), r.i(0), r.i(1)).map(|v| Record {
            floats: v
                .values
                .into_iter()
                .chain([
                    v.semi_major_axis_au(),
                    v.eccentricity(),
                    v.inclination_deg(),
                    v.ascending_node_deg(),
                    v.argument_of_perihelion_deg(),
                    v.longitude_of_periapsis_deg(),
                    v.mean_anomaly_deg(),
                    v.true_anomaly_deg(),
                    v.eccentric_anomaly_deg(),
                    v.mean_longitude_deg(),
                    v.sidereal_period_years(),
                    v.mean_daily_motion_deg(),
                    v.tropical_period_years(),
                    v.synodic_period_days(),
                    v.perihelion_passage_jd(),
                    v.perihelion_distance_au(),
                    v.aphelion_distance_au(),
                ])
                .collect(),
            texts: vec![v.diagnostic.into_bytes()],
            ..Record::default()
        }),
        OrbitMaxMinTrueDistance => {
            orbit_max_min_true_distance(r.f(0), r.i(0), r.i(1)).map(|v| Record {
                floats: vec![v.max_distance, v.min_distance, v.true_distance],
                texts: vec![v.diagnostic.into_bytes()],
                ..Record::default()
            })
        }
        _ => unreachable!("physical operation validated by schema"),
    }
}
