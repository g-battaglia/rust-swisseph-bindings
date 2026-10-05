/* Heliacal/default-writing inputs belong to this call only. Guards surround
 * every writable array, including defaults and the full in/out object name.
 * The known unsafe AVKIND search paths are never dispatched by the driver. */
case OP_HELIACAL_UT: case OP_HELIACAL_PHENO_UT: case OP_VIS_LIMIT_MAG: {
    if (request->op == OP_VIS_LIMIT_MAG) { SHAPE(1, 14, 1); } else { SHAPE(2, 14, 1); }
    DBUF(geography, 3); DBUF(atmosphere, 4); DBUF(observer, 6);
    for (unsigned i = 0; i < 3; ++i) geography.v[i] = F(i + 1);
    for (unsigned i = 0; i < 4; ++i) atmosphere.v[i] = F(i + 4);
    for (unsigned i = 0; i < 6; ++i) observer.v[i] = F(i + 8);
    TBUF(name, SE_MAX_STNAME); TBUF(diagnostic, 256);
    copy_query(name.v, sizeof(name.v), T(0));
    if (request->op == OP_HELIACAL_UT) {
        DBUF(out, 10); result.code = swe_heliacal_ut(F(0), geography.v, atmosphere.v, observer.v, name.v, I(0), I(1), out.v, diagnostic.v); DOUBLES(out);
    } else if (request->op == OP_HELIACAL_PHENO_UT) {
        DBUF(out, 30); result.code = swe_heliacal_pheno_ut(F(0), geography.v, atmosphere.v, observer.v, name.v, I(0), I(1), out.v, diagnostic.v); DOUBLES(out);
    } else {
        DBUF(out, 8); result.code = swe_vis_limit_mag(F(0), geography.v, atmosphere.v, observer.v, name.v, I(0), out.v, diagnostic.v); DOUBLES(out);
    }
    CHECK(geography); CHECK(atmosphere); CHECK(observer); CHECK(name); TEXT(diagnostic); break;
}
