//! Safe public-API worker. No raw C declarations or production helpers are used.
//!
//! Wire arguments follow the operation schema in `schema.rs`; result fields are
//! copied independently in their public order. Errors contain owned text only.
use super::protocol::{Record, Request, read_frame, write_frame};
use super::registry::Op;
use std::io;
use swisseph_bindings::*;

/// Worker-only status distinct from all native statuses, including no-event.
pub const NATIVE_ERROR: i32 = -101;
/// Worker-only Rust input rejection; never confused with a native failure.
pub const INVALID_INPUT: i32 = -100;
/// Worker-only lock failure; a poisoned configuration is not numerical data.
pub const LOCK_ERROR: i32 = -102;

/// Execute framed requests until clean EOF; malformed input fails the child.
pub fn run() -> Result<(), String> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    // Source-policy calls consume the last genuine public position, not a
    // reconstructed/fabricated flag record. Failed calls leave it unchanged.
    let mut last_position = None;
    let mut sessions = super::session_probe::State::default();
    while let Some(frame) = read_frame(&mut input).map_err(|e| e.to_string())? {
        let request = Request::decode(&frame).map_err(|e| e.to_string())?;
        if matches!(request.op, 1000 | 1001)
            && !(request.ints.is_empty() && request.floats.is_empty() && request.texts.is_empty())
        {
            return Err("reserved request argument shape".into());
        }
        let mut record = if request.op == 1000 {
            Record {
                ints: vec![4, 8, 1],
                texts: vec![version().map_err(|e| e.to_string())?.into_bytes()],
                ..Record::default()
            }
        } else if request.op == 1001 {
            super::constants::rust_constants()
        } else if request.op == super::session_probe::PROBE {
            super::session_probe::run(&request, &mut sessions)?
        } else if request.op == super::session_probe::PROVENANCE {
            super::session_probe::provenance(&request, &sessions)?
        } else if request.op == super::session_rejection::REJECT {
            super::session_rejection::run(&request, &sessions)?
        } else if (super::session_commands::BUILD..=super::session_commands::DROP)
            .contains(&request.op)
        {
            super::session_commands::dispatch(&request, &mut sessions)?
        } else {
            let op = Op::from_wire(request.op).ok_or("unknown public operation")?;
            super::schema::validate(op, &request)?;
            if op == Op::RequireSourceFlags && last_position.is_none() {
                return Err("source policy requires an earlier successful position".into());
            }
            match call(op, &request, &mut last_position) {
                Ok(record) => record,
                Err(value) => error(value),
            }
        };
        record.id = request.id;
        record.op = request.op;
        write_frame(&mut output, &record.encode()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Preserve the actual public error class and complete owned message. This
/// shared Rust-side packer is never a C projector or scientific reference.
pub(super) fn error(value: Error) -> Record {
    Record {
        code: match value.kind() {
            ErrorKind::InvalidInput => INVALID_INPUT,
            ErrorKind::Native => NATIVE_ERROR,
            ErrorKind::LockPoisoned => LOCK_ERROR,
        },
        texts: vec![value.message().as_bytes().to_vec()],
        ..Record::default()
    }
}

/// An exact scalar with no conversion or diagnostic invented by the worker.
pub(super) fn scalar(value: f64) -> Record {
    Record {
        floats: vec![value],
        ..Record::default()
    }
}
/// An exact native integer result, distinct from the status channel.
fn integer(value: i32) -> Record {
    Record {
        ints: vec![value],
        ..Record::default()
    }
}
/// An already-owned public string copied as UTF-8 bytes.
fn text(value: String) -> Record {
    Record {
        texts: vec![value.into_bytes()],
        ..Record::default()
    }
}
/// A public value plus its entire native diagnostic.
fn scalar_diagnostic(value: f64, diagnostic: String) -> Record {
    Record {
        floats: vec![value],
        texts: vec![diagnostic.into_bytes()],
        ..Record::default()
    }
}
/// All public position components and actual returned flags, never requested flags.
pub(super) fn position(value: Position, last: &mut Option<Position>) -> Record {
    *last = Some(value.clone());
    Record {
        code: value.returned_flags,
        // Accessor outputs are separately exposed and checked against native
        // slots by the driver; matching only the backing array is insufficient.
        floats: value
            .values
            .into_iter()
            .chain([value.longitude(), value.latitude(), value.distance()])
            .collect(),
        texts: vec![value.diagnostic.into_bytes()],
        ..Record::default()
    }
}
/// A calendar value; the fractional hour remains a floating result.
fn date(value: CalendarDate) -> Record {
    Record {
        ints: vec![value.year, value.month, value.day],
        floats: vec![value.hour],
        ..Record::default()
    }
}
/// A UTC value; integer clock components are not rounded from seconds.
fn utc(value: UtcDateTime) -> Record {
    Record {
        ints: vec![value.year, value.month, value.day, value.hour, value.minute],
        floats: vec![value.second],
        ..Record::default()
    }
}
/// Translate only the two representable public calendar variants.
fn calendar(flag: i32) -> Calendar {
    match flag {
        0 => Calendar::Julian,
        1 => Calendar::Gregorian,
        _ => unreachable!("validated calendar"),
    }
}
/// Public cusp/angle order contains no native index-zero padding.
pub(super) fn house(value: Houses) -> Record {
    Record {
        floats: value
            .cusps
            .into_iter()
            .chain(value.angles)
            .chain([value.ascendant(), value.mc()])
            .collect(),
        ..Record::default()
    }
}
/// Public speed order is cusps, angles, cusp speeds, angle speeds.
fn house_speeds(value: HousesWithSpeeds) -> Record {
    Record {
        floats: value
            .cusps
            .into_iter()
            .chain(value.angles)
            .chain(value.cusp_speeds)
            .chain(value.angle_speeds)
            .chain([value.ascendant(), value.mc()])
            .collect(),
        ..Record::default()
    }
}

/// Dispatch only implemented operations after exact shape validation.
fn call(op: Op, r: &Request, last: &mut Option<Position>) -> Result<Record, Error> {
    use Op::*;
    match op {
        Version => version().map(text),
        LibraryPath => library_path().map(text),
        GetPlanetName => get_planet_name(r.i(0)).map(text),
        HouseName => Ok(text(house_name(r.i(0) as u8))),
        GetAyanamsaName => get_ayanamsa_name(r.i(0)).map(text),
        Julday => Ok(scalar(julday(
            r.i(0),
            r.i(1),
            r.i(2),
            r.f(0),
            calendar(r.i(3)),
        ))),
        Revjul => revjul(r.f(0), calendar(r.i(0))).map(date),
        UtcToJd => utc_to_jd(
            r.i(0),
            r.i(1),
            r.i(2),
            r.i(3),
            r.i(4),
            r.f(0),
            calendar(r.i(5)),
        )
        .map(|v| Record {
            floats: vec![v.jd_et, v.jd_ut],
            ..Record::default()
        }),
        JdetToUtc => jdet_to_utc(r.f(0), calendar(r.i(0))).map(utc),
        Jdut1ToUtc => jdut1_to_utc(r.f(0), calendar(r.i(0))).map(utc),
        UtcTimeZone => {
            utc_time_zone(r.i(0), r.i(1), r.i(2), r.i(3), r.i(4), r.f(0), r.f(1)).map(utc)
        }
        DateConversion => {
            date_conversion(r.i(0), r.i(1), r.i(2), r.f(0), calendar(r.i(3))).map(scalar)
        }
        DayOfWeek => day_of_week(r.f(0)).map(integer),
        Deltat => deltat(r.f(0)).map(scalar),
        DeltatEx => deltat_ex(r.f(0), r.i(0)).map(|v| scalar_diagnostic(v.value, v.diagnostic)),
        TimeEqu => time_equ(r.f(0)).map(|v| scalar_diagnostic(v.value, v.diagnostic)),
        LmtToLat => lmt_to_lat(r.f(0), r.f(1)).map(|v| scalar_diagnostic(v.jd, v.diagnostic)),
        LatToLmt => lat_to_lmt(r.f(0), r.f(1)).map(|v| scalar_diagnostic(v.jd, v.diagnostic)),
        Sidtime => sidtime(r.f(0)).map(scalar),
        Sidtime0 => sidtime0(r.f(0), r.f(1), r.f(2)).map(scalar),
        CalcUt => calc_ut(r.f(0), r.i(0), r.i(1)).map(|value| position(value, last)),
        Calc => calc(r.f(0), r.i(0), r.i(1)).map(|value| position(value, last)),
        CalcPctr => calc_pctr(r.f(0), r.i(0), r.i(1), r.i(2)).map(|value| position(value, last)),
        RequireSourceFlags => require_source_flags(
            last.as_ref()
                .expect("source policy needs a preceding successful position"),
            r.i(0),
        )
        .map(|()| Record::default()),
        SetEphePath => set_ephe_path(r.optional_text(0)).map(|()| Record::default()),
        SetJplFile => set_jpl_file(r.text(0)).map(|()| Record::default()),
        SetTopo => set_topo(r.f(0), r.f(1), r.f(2)).map(|()| Record::default()),
        SetSidMode => set_sid_mode(r.i(0), r.f(0), r.f(1)).map(|()| Record::default()),
        SetTidAcc => set_tid_acc(r.f(0)).map(|()| Record::default()),
        GetTidAcc => get_tid_acc().map(scalar),
        SetDeltaTUserdef => set_delta_t_userdef(if r.i(0) == 0 { None } else { Some(r.f(0)) })
            .map(|()| Record::default()),
        SetLapseRate => set_lapse_rate(r.f(0)).map(|()| Record::default()),
        Close => close().map(|()| Record::default()),
        GetAyanamsa => get_ayanamsa(r.f(0)).map(scalar),
        GetAyanamsaUt => get_ayanamsa_ut(r.f(0)).map(scalar),
        GetAyanamsaEx | GetAyanamsaExUt => {
            let value = if op == GetAyanamsaEx {
                get_ayanamsa_ex(r.f(0), r.i(0))
            } else {
                get_ayanamsa_ex_ut(r.f(0), r.i(0))
            }?;
            Ok(Record {
                code: value.returned_flags,
                floats: vec![value.value],
                texts: vec![value.diagnostic.into_bytes()],
                ..Record::default()
            })
        }
        Houses => houses(r.f(0), r.f(1), r.f(2), r.i(0) as u8).map(house),
        HousesEx => houses_ex(r.f(0), r.i(0), r.f(1), r.f(2), r.i(1) as u8).map(house),
        HousesEx2 => houses_ex2(r.f(0), r.i(0), r.f(1), r.f(2), r.i(1) as u8).map(house_speeds),
        HousesArmc => houses_armc(r.f(0), r.f(1), r.f(2), r.i(0) as u8).map(house),
        HousesArmcEx2 => {
            houses_armc_ex2(r.f(0), r.f(1), r.f(2), r.i(0) as u8, r.f(3)).map(house_speeds)
        }
        HousesGauquelin => houses_gauquelin(r.f(0), r.i(0), r.f(1), r.f(2)).map(|v| Record {
            floats: v
                .sectors
                .into_iter()
                .chain(v.angles)
                .chain(v.sector_speeds)
                .chain(v.angle_speeds)
                .collect(),
            ..Record::default()
        }),
        HousePos => house_pos(r.f(0), r.f(1), r.f(2), r.i(0) as u8, r.f(3), r.f(4))
            .map(|v| scalar_diagnostic(v.value, v.diagnostic)),
        GetCurrentFileData => get_current_file_data(r.i(0)).map(|v| match v {
            None => Record {
                ints: vec![0],
                ..Record::default()
            },
            Some(v) => Record {
                ints: vec![1, v.denum],
                floats: vec![v.tfstart, v.tfend],
                texts: vec![v.path.into_bytes()],
                ..Record::default()
            },
        }),
        Cotrans => cotrans(r.f(0), r.f(1), r.f(2), r.f(3)).map(|v| Record {
            floats: v.to_vec(),
            ..Record::default()
        }),
        CotransSp => {
            cotrans_sp(r.f(0), r.f(1), r.f(2), r.f(3), r.f(4), r.f(5), r.f(6)).map(|(p, v)| {
                Record {
                    floats: p.into_iter().chain(v).collect(),
                    ..Record::default()
                }
            })
        }
        Degnorm => Ok(scalar(degnorm(r.f(0)))),
        Radnorm => Ok(scalar(radnorm(r.f(0)))),
        Difdegn => Ok(scalar(difdegn(r.f(0), r.f(1)))),
        Difdeg2n => Ok(scalar(difdeg2n(r.f(0), r.f(1)))),
        Difrad2n => Ok(scalar(difrad2n(r.f(0), r.f(1)))),
        DegMidp => Ok(scalar(deg_midp(r.f(0), r.f(1)))),
        RadMidp => Ok(scalar(rad_midp(r.f(0), r.f(1)))),
        D2l => d2l(r.f(0)).map(integer),
        Csnorm => Ok(integer(csnorm(r.i(0)))),
        Difcsn => difcsn(r.i(0), r.i(1)).map(integer),
        Difcs2n => difcs2n(r.i(0), r.i(1)).map(integer),
        Csroundsec => csroundsec(r.i(0)).map(integer),
        Cs2timestr => cs2timestr(r.i(0), r.i(1) as u8, r.i(2) != 0).map(text),
        Cs2lonlatstr => cs2lonlatstr(r.i(0), r.i(1) as u8, r.i(2) as u8).map(text),
        Cs2degstr => cs2degstr(r.i(0)).map(text),
        SplitDeg => split_deg(r.f(0), r.i(0)).map(|v| Record {
            ints: vec![v.deg, v.min, v.sec, v.sign],
            floats: vec![v.secfr],
            ..Record::default()
        }),
        Fixstar
        | FixstarUt
        | Fixstar2
        | Fixstar2Ut
        | FixstarMag
        | Fixstar2Mag
        | NodAps
        | NodApsUt
        | RiseTrans
        | RiseTransTrueHor
        | GauquelinSector
        | Azalt
        | AzaltRev
        | Refrac
        | RefracExtended
        | Pheno
        | PhenoUt
        | GetOrbitalElements
        | OrbitMaxMinTrueDistance => super::physical_worker::call(op, r),
        Solcross | SolcrossUt | Mooncross | MooncrossUt | MooncrossNode | MooncrossNodeUt
        | HelioCross | HelioCrossUt => super::crossing_worker::call(op, r),
        SolEclipseWhenGlob
        | LunEclipseWhen
        | SolEclipseWhere
        | SolEclipseHow
        | LunEclipseHow
        | SolEclipseWhenLoc
        | LunEclipseWhenLoc
        | LunOccultWhenGlob
        | LunOccultWhenGlobWithOptions
        | LunOccultWhenLoc
        | LunOccultWhenLocWithOptions
        | LunOccultWhere => super::eclipse_worker::call(op, r),
        HeliacalUt | HeliacalPhenoUt | VisLimitMag => super::heliacal_worker::call(op, r),
    }
}
