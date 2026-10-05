/* Public-header star/observer calls. Included only inside dispatch's switch.
 * Units/order are recorded in physical_worker.rs and the public API contract.
 * Star in/out capacity is twice SE_MAX_STNAME; observer names match the safe
 * wrapper's single-SE_MAX_STNAME capacity. Nothing borrows native scratch. */
case OP_FIXSTAR: case OP_FIXSTAR_UT: case OP_FIXSTAR2: case OP_FIXSTAR2_UT:
case OP_FIXSTAR_MAG: case OP_FIXSTAR2_MAG: {
    const int magnitude = request->op == OP_FIXSTAR_MAG || request->op == OP_FIXSTAR2_MAG;
    const int extended = request->op == OP_FIXSTAR2 || request->op == OP_FIXSTAR2_UT || request->op == OP_FIXSTAR2_MAG;
    if (magnitude) { SHAPE(0, 0, 1); } else { SHAPE(1, 1, 1); }
    TBUF(name, 2 * SE_MAX_STNAME); TBUF(diagnostic, 256);
    DBUF(position, 6); DBUF(mag, 1);
    copy_query(name.v, sizeof(name.v), T(0));
    if (extended) {
        /* A missing catalog makes extended native lookup unsafe at this pin.
         * The legacy magnitude preflight is an explicitly named reference
         * failure, independently reproducing the safe public contract. */
        TBUF(probe, 2 * SE_MAX_STNAME); DBUF(probe_magnitude, 1);
        probe.v[0] = '1';
        result.code = swe_fixstar_mag(probe.v, probe_magnitude.v, diagnostic.v);
        CHECK(probe); CHECK(probe_magnitude);
        if (result.code < 0) {
            /* C-only routing metadata: do not count an uncalled extended
             * symbol as exercised when only the safe preflight ran. */
            add_int(&result, OP_FIXSTAR_MAG); TEXT(diagnostic); break;
        }
        memset(diagnostic.v, 0, sizeof(diagnostic.v));
    }
    switch (request->op) {
    case OP_FIXSTAR: result.code = swe_fixstar(name.v, F(0), I(0), position.v, diagnostic.v); break;
    case OP_FIXSTAR_UT: result.code = swe_fixstar_ut(name.v, F(0), I(0), position.v, diagnostic.v); break;
    case OP_FIXSTAR2: result.code = swe_fixstar2(name.v, F(0), I(0), position.v, diagnostic.v); break;
    case OP_FIXSTAR2_UT: result.code = swe_fixstar2_ut(name.v, F(0), I(0), position.v, diagnostic.v); break;
    case OP_FIXSTAR_MAG: result.code = swe_fixstar_mag(name.v, mag.v, diagnostic.v); break;
    default: result.code = swe_fixstar2_mag(name.v, mag.v, diagnostic.v); break;
    }
    CHECK(position); CHECK(mag);
    if (magnitude) { DOUBLES(mag); } else { DOUBLES(position); }
    TEXT(name); TEXT(diagnostic); break;
}
case OP_NOD_APS: case OP_NOD_APS_UT: {
    SHAPE(3, 1, 0); DBUF(ascending, 6); DBUF(descending, 6); DBUF(perihelion, 6); DBUF(aphelion, 6); TBUF(diagnostic, 256);
    if (request->op == OP_NOD_APS) result.code = swe_nod_aps(F(0), I(0), I(1), I(2), ascending.v, descending.v, perihelion.v, aphelion.v, diagnostic.v);
    else result.code = swe_nod_aps_ut(F(0), I(0), I(1), I(2), ascending.v, descending.v, perihelion.v, aphelion.v, diagnostic.v);
    DOUBLES(ascending); DOUBLES(descending); DOUBLES(perihelion); DOUBLES(aphelion); TEXT(diagnostic); break;
}
case OP_RISE_TRANS: case OP_RISE_TRANS_TRUE_HOR: case OP_GAUQUELIN_SECTOR: {
    if (request->op == OP_RISE_TRANS_TRUE_HOR) { SHAPE(3, 7, 1); } else { SHAPE(3, 6, 1); }
    double geopos[3] = {F(1), F(2), F(3)};
    TBUF(name, SE_MAX_STNAME); TBUF(diagnostic, 256); DBUF(out, 1);
    if (T(0)) copy_query(name.v, sizeof(name.v), T(0));
    char *star = T(0) ? name.v : NULL;
    if (request->op == OP_RISE_TRANS) result.code = swe_rise_trans(F(0), I(0), star, I(1), I(2), geopos, F(4), F(5), out.v, diagnostic.v);
    else if (request->op == OP_RISE_TRANS_TRUE_HOR) result.code = swe_rise_trans_true_hor(F(0), I(0), star, I(1), I(2), geopos, F(4), F(5), F(6), out.v, diagnostic.v);
    else result.code = swe_gauquelin_sector(F(0), I(0), star, I(1), I(2), geopos, F(4), F(5), out.v, diagnostic.v);
    CHECK(name); DOUBLES(out); TEXT(diagnostic); break;
}
case OP_AZALT: case OP_AZALT_REV: {
    if (request->op == OP_AZALT) { SHAPE(1, 8, 0); } else { SHAPE(1, 6, 0); }
    double geopos[3] = {F(1), F(2), F(3)};
    if (request->op == OP_AZALT) {
        double input[3] = {F(6), F(7), 0}; DBUF(out, 3);
        swe_azalt(F(0), I(0), geopos, F(4), F(5), input, out.v); DOUBLES(out);
    } else {
        double input[3] = {F(4), F(5), 0}; DBUF(out, 2);
        swe_azalt_rev(F(0), I(0), geopos, input, out.v); DOUBLES(out);
    }
    break;
}
case OP_REFRAC: {
    SHAPE(1, 3, 0); add_float(&result, swe_refrac(F(0), F(1), F(2), I(0))); break;
}
case OP_REFRAC_EXTENDED: {
    SHAPE(1, 5, 0); DBUF(details, 4);
    add_float(&result, swe_refrac_extended(F(0), F(1), F(2), F(3), F(4), I(0), details.v)); DOUBLES(details); break;
}
case OP_PHENO: case OP_PHENO_UT: case OP_GET_ORBITAL_ELEMENTS: case OP_ORBIT_MAX_MIN_TRUE_DISTANCE: {
    SHAPE(2, 1, 0); TBUF(diagnostic, 256);
    if (request->op == OP_GET_ORBITAL_ELEMENTS) {
        DBUF(out, 50); result.code = swe_get_orbital_elements(F(0), I(0), I(1), out.v, diagnostic.v); DOUBLES(out);
    } else if (request->op == OP_ORBIT_MAX_MIN_TRUE_DISTANCE) {
        DBUF(out, 3); result.code = swe_orbit_max_min_true_distance(F(0), I(0), I(1), &out.v[0], &out.v[1], &out.v[2], diagnostic.v); DOUBLES(out);
    } else {
        DBUF(out, 20);
        if (request->op == OP_PHENO) result.code = swe_pheno(F(0), I(0), I(1), out.v, diagnostic.v);
        else result.code = swe_pheno_ut(F(0), I(0), I(1), out.v, diagnostic.v);
        DOUBLES(out);
    }
    TEXT(diagnostic); break;
}
