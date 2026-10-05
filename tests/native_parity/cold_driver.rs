//! Fresh-process native defaults and first-write/reset state comparisons.
//! Each recipe owns a separate C/Rust worker pair, with no scientific calls
//! before the recipe. Native outputs stay in memory; only field counts survive.
use super::cases::step;
use super::process::{Role, Worker};
use super::protocol::Request;
use super::registry::Op;
use std::path::Path;
use std::time::Duration;
use swisseph_bindings::*;

/// Construct ordinary safe public-ABI operations for a fresh worker pair.
fn recipes(empty: &str) -> Vec<(&'static str, Vec<Request>)> {
    use Op::*;
    let scalar = |op| step(op, &[], &[2451545.0], &[]).request;
    let mut observe = vec![
        step(GetTidAcc, &[], &[], &[]).request,
        scalar(Deltat),
        step(GetTidAcc, &[], &[], &[]).request,
        scalar(GetAyanamsaUt),
        scalar(Sidtime),
        super::session_driver::input(Houses, FLG_MOSEPH),
    ];
    // A subsequent explicit source must not freeze automatic tidal selection.
    observe.push(
        step(
            CalcUt,
            &[MOON, FLG_MOSEPH | FLG_SPEED | FLG_SIDEREAL],
            &[2451545.0],
            &[],
        )
        .request,
    );
    observe.push(step(GetTidAcc, &[], &[], &[]).request);
    let mut tidal = vec![step(SetTidAcc, &[], &[-23.75], &[]).request];
    tidal.extend(observe.clone());
    let mut delta = vec![step(SetDeltaTUserdef, &[1], &[0.0015], &[]).request];
    delta.extend(observe.clone());
    let mut reset = vec![
        step(SetEphePath, &[], &[], &[Some(empty)]).request,
        step(SetTopo, &[], &[151.21, -33.87, 50.0], &[]).request,
        step(SetSidMode, &[SIDM_LAHIRI], &[0.0, 0.0], &[]).request,
        step(SetTidAcc, &[], &[-23.75], &[]).request,
        step(SetDeltaTUserdef, &[1], &[0.0015], &[]).request,
    ];
    reset.extend(observe.clone());
    reset.push(step(Close, &[], &[], &[]).request);
    reset.extend(observe.clone());
    reset.push(step(SetDeltaTUserdef, &[0], &[0.0], &[]).request);
    reset.extend(observe.clone());
    reset.push(step(SetTidAcc, &[], &[SE_TIDAL_AUTOMATIC], &[]).request);
    reset.extend(observe.clone());
    vec![
        ("COLD_DEFAULTS", observe),
        ("COLD_FIRST_TIDAL_WRITE", tidal),
        ("COLD_FIRST_DELTA_WRITE", delta),
        ("COLD_CLOSE_CLEAR_AUTOMATIC", reset),
    ]
}

/// Validate four independent cold/reset scripts. Compiled default-path probes
/// require the pinned Unix absolute default directories to be absent, otherwise
/// their unmanifested contents would invalidate the file-independent claim.
pub fn run(
    oracle: &Path,
    executable: &Path,
    cwd: &Path,
    empty: &str,
    deadline: Duration,
) -> Result<(usize, usize), String> {
    if Path::new("/users/ephe2").exists() || Path::new("/users/ephe").exists() {
        return Err(
            "cold/default-path prerequisite: unmanifested compiled-default data directory exists"
                .into(),
        );
    }
    let recipes = recipes(empty);
    let mut fields = 0;
    for (key, requests) in &recipes {
        let mut c = Worker::start(oracle, Role::Native, cwd, deadline)?;
        let mut rust = Worker::start(executable, Role::Rust, cwd, deadline)?;
        for (index, input) in requests.iter().enumerate() {
            let mut input = input.clone();
            input.id = index as u32 + 1;
            let op = Op::from_wire(input.op).ok_or("cold operation identity")?;
            let context = |failure| {
                format!(
                    "case={key} step={index} operation={}: {failure}",
                    op.entry().name
                )
            };
            let native = c.request(&input).map_err(context)?;
            let record = rust.request(&input).map_err(context)?;
            fields +=
                super::compare::operation(op, &input, Some(&native), &record).map_err(context)?;
        }
        c.finish()?;
        rust.finish()?;
    }
    Ok((recipes.len(), fields))
}
