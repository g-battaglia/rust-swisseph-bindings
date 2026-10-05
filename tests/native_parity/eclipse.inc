/* Independent public-header eclipse/occultation caller.
 * Capacities: global/lunar/occultation times 10; local solar times 7;
 * geography output 2; observer input 3; every attribute array 20.
 * One-try is a bit in the search-control integer, not an ephemeris flag. */
case OP_SOL_ECLIPSE_WHEN_GLOB: case OP_LUN_ECLIPSE_WHEN: {
    SHAPE(3, 1, 0); DBUF(times, 10); TBUF(diagnostic, 256);
    if (request->op == OP_SOL_ECLIPSE_WHEN_GLOB) result.code = swe_sol_eclipse_when_glob(F(0), I(0), I(1), times.v, I(2), diagnostic.v);
    else result.code = swe_lun_eclipse_when(F(0), I(0), I(1), times.v, I(2), diagnostic.v);
    DOUBLES(times); TEXT(diagnostic); break;
}
case OP_SOL_ECLIPSE_WHERE: case OP_LUN_OCCULT_WHERE: {
    TBUF(name, SE_MAX_STNAME); DBUF(geography, 2); DBUF(attributes, 20); TBUF(diagnostic, 256);
    if (request->op == OP_SOL_ECLIPSE_WHERE) {
        SHAPE(1, 1, 0); result.code = swe_sol_eclipse_where(F(0), I(0), geography.v, attributes.v, diagnostic.v);
    } else {
        SHAPE(2, 1, 1); if (T(0)) copy_query(name.v, sizeof(name.v), T(0));
        result.code = swe_lun_occult_where(F(0), I(0), T(0) ? name.v : NULL, I(1), geography.v, attributes.v, diagnostic.v);
    }
    CHECK(name); DOUBLES(geography); DOUBLES(attributes); TEXT(diagnostic); break;
}
case OP_SOL_ECLIPSE_HOW: case OP_LUN_ECLIPSE_HOW: {
    if (request->op == OP_SOL_ECLIPSE_HOW) { SHAPE(1, 4, 0); } else { SHAPE(2, 4, 0); }
    double observer[3] = {F(1), F(2), F(3)}; DBUF(attributes, 20); TBUF(diagnostic, 256);
    if (request->op == OP_SOL_ECLIPSE_HOW) result.code = swe_sol_eclipse_how(F(0), I(0), observer, attributes.v, diagnostic.v);
    else result.code = swe_lun_eclipse_how(F(0), I(0), I(1) ? observer : NULL, attributes.v, diagnostic.v);
    DOUBLES(attributes); TEXT(diagnostic); break;
}
case OP_SOL_ECLIPSE_WHEN_LOC: case OP_LUN_ECLIPSE_WHEN_LOC: {
    SHAPE(2, 4, 0); double observer[3] = {F(1), F(2), F(3)}; DBUF(attributes, 20); TBUF(diagnostic, 256);
    if (request->op == OP_SOL_ECLIPSE_WHEN_LOC) {
        DBUF(times, 7); result.code = swe_sol_eclipse_when_loc(F(0), I(0), observer, times.v, attributes.v, I(1), diagnostic.v); DOUBLES(times);
    } else {
        DBUF(times, 10); result.code = swe_lun_eclipse_when_loc(F(0), I(0), observer, times.v, attributes.v, I(1), diagnostic.v); DOUBLES(times);
    }
    DOUBLES(attributes); TEXT(diagnostic); break;
}
case OP_LUN_OCCULT_WHEN_GLOB: case OP_LUN_OCCULT_WHEN_GLOB_WITH_OPTIONS: {
    const int extended = request->op == OP_LUN_OCCULT_WHEN_GLOB_WITH_OPTIONS;
    if (extended) { SHAPE(5, 1, 1); } else { SHAPE(4, 1, 1); }
    TBUF(name, SE_MAX_STNAME); DBUF(times, 10); TBUF(diagnostic, 256);
    if (T(0)) copy_query(name.v, sizeof(name.v), T(0));
    const int control = I(3) | (extended && I(4) ? SE_ECL_ONE_TRY : 0);
    result.code = swe_lun_occult_when_glob(F(0), I(0), T(0) ? name.v : NULL, I(1), I(2), times.v, control, diagnostic.v);
    CHECK(name); DOUBLES(times); TEXT(diagnostic); break;
}
case OP_LUN_OCCULT_WHEN_LOC: case OP_LUN_OCCULT_WHEN_LOC_WITH_OPTIONS: {
    const int extended = request->op == OP_LUN_OCCULT_WHEN_LOC_WITH_OPTIONS;
    if (extended) { SHAPE(4, 4, 1); } else { SHAPE(3, 4, 1); }
    double observer[3] = {F(1), F(2), F(3)};
    TBUF(name, SE_MAX_STNAME); DBUF(times, 10); DBUF(attributes, 20); TBUF(diagnostic, 256);
    if (T(0)) copy_query(name.v, sizeof(name.v), T(0));
    const int control = I(2) | (extended && I(3) ? SE_ECL_ONE_TRY : 0);
    result.code = swe_lun_occult_when_loc(F(0), I(0), T(0) ? name.v : NULL, I(1), observer, times.v, attributes.v, control, diagnostic.v);
    CHECK(name); DOUBLES(times); DOUBLES(attributes); TEXT(diagnostic); break;
}
