//! Declaration-level request shapes, independent of native output projectors.
use super::protocol::Request;
use super::registry::Op;

/// Exhaustive exact argument counts: integers, doubles, optional text slots.
/// New operations require an explicit shape; no default can silently accept them.
pub fn shape(op: Op) -> (usize, usize, usize) {
    use Op::*;
    match op {
        Version | LibraryPath | GetTidAcc | Close => (0, 0, 0),
        GetPlanetName | HouseName | GetAyanamsaName | Csnorm | Csroundsec | Cs2degstr
        | RequireSourceFlags | GetCurrentFileData => (1, 0, 0),
        Julday | DateConversion => (4, 1, 0),
        Revjul | JdetToUtc | Jdut1ToUtc | GetAyanamsaEx | GetAyanamsaExUt | DeltatEx | SplitDeg
        | SetDeltaTUserdef => (1, 1, 0),
        UtcToJd => (6, 1, 0),
        UtcTimeZone => (5, 2, 0),
        DayOfWeek | Deltat | TimeEqu | Sidtime | GetAyanamsa | GetAyanamsaUt | SetTidAcc
        | SetLapseRate | Degnorm | Radnorm | D2l => (0, 1, 0),
        LmtToLat | LatToLmt | Difdegn | Difdeg2n | Difrad2n | DegMidp | RadMidp => (0, 2, 0),
        Sidtime0 | SetTopo => (0, 3, 0),
        Calc | CalcUt => (2, 1, 0),
        CalcPctr => (3, 1, 0),
        SetEphePath | SetJplFile => (0, 0, 1),
        SetSidMode => (1, 2, 0),
        Houses | HousesArmc | HousesGauquelin => (1, 3, 0),
        HousesEx | HousesEx2 => (2, 3, 0),
        HousesArmcEx2 => (1, 4, 0),
        HousePos => (1, 5, 0),
        Cotrans => (0, 4, 0),
        CotransSp => (0, 7, 0),
        Difcsn | Difcs2n => (2, 0, 0),
        Cs2timestr | Cs2lonlatstr => (3, 0, 0),
        Fixstar | FixstarUt | Fixstar2 | Fixstar2Ut => (1, 1, 1),
        FixstarMag | Fixstar2Mag => (0, 0, 1),
        NodAps | NodApsUt => (3, 1, 0),
        RiseTrans | GauquelinSector => (3, 6, 1),
        RiseTransTrueHor => (3, 7, 1),
        Azalt => (1, 8, 0),
        AzaltRev => (1, 6, 0),
        Refrac => (1, 3, 0),
        RefracExtended => (1, 5, 0),
        Pheno | PhenoUt | GetOrbitalElements | OrbitMaxMinTrueDistance => (2, 1, 0),
        Solcross | SolcrossUt | Mooncross | MooncrossUt => (1, 2, 0),
        MooncrossNode | MooncrossNodeUt => (1, 1, 0),
        HelioCross | HelioCrossUt => (3, 2, 0),
        SolEclipseWhenGlob | LunEclipseWhen => (3, 1, 0),
        SolEclipseWhere => (1, 1, 0),
        SolEclipseHow => (1, 4, 0),
        LunEclipseHow | SolEclipseWhenLoc | LunEclipseWhenLoc => (2, 4, 0),
        LunOccultWhenGlob => (4, 1, 1),
        LunOccultWhenGlobWithOptions => (5, 1, 1),
        LunOccultWhenLoc => (3, 4, 1),
        LunOccultWhenLocWithOptions => (4, 4, 1),
        LunOccultWhere => (2, 1, 1),
        HeliacalUt | HeliacalPhenoUt => (2, 14, 1),
        VisLimitMag => (1, 14, 1),
    }
}

/// Reject malformed requests before indexing; calendar variants cannot be guessed.
pub fn validate(op: Op, request: &Request) -> Result<(), String> {
    let expected = shape(op);
    if expected
        != (
            request.ints.len(),
            request.floats.len(),
            request.texts.len(),
        )
    {
        return Err(format!("argument shape for {}", op.entry().name));
    }
    use Op::*;
    let calendar = match op {
        Julday | DateConversion => Some(3),
        UtcToJd => Some(5),
        Revjul | JdetToUtc | Jdut1ToUtc => Some(0),
        _ => None,
    };
    if calendar.is_some_and(|index| !matches!(request.ints[index], 0 | 1)) {
        return Err("unrepresentable calendar variant".into());
    }
    if matches!(
        op,
        Fixstar | FixstarUt | Fixstar2 | Fixstar2Ut | FixstarMag | Fixstar2Mag
    ) && request.texts[0].is_none()
    {
        return Err("star query argument is not optional".into());
    }
    if matches!(op, HeliacalUt | HeliacalPhenoUt | VisLimitMag) && request.texts[0].is_none() {
        return Err("heliacal object argument is not optional".into());
    }
    if op == SetJplFile && request.texts[0].is_none() {
        return Err("JPL file argument is not optional".into());
    }
    Ok(())
}
