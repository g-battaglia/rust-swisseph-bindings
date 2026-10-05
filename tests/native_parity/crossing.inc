/* Native public-ABI searches only; no root finding or result normalization.
 * Double-returning functions retain their raw sentinel in the float channel;
 * the driver independently classifies it against the original input epoch. */
case OP_SOLCROSS: case OP_SOLCROSS_UT: case OP_MOONCROSS: case OP_MOONCROSS_UT: {
    SHAPE(1, 2, 0); TBUF(diagnostic, 256); double time = 0;
    switch (request->op) {
    case OP_SOLCROSS: time = swe_solcross(F(0), F(1), I(0), diagnostic.v); break;
    case OP_SOLCROSS_UT: time = swe_solcross_ut(F(0), F(1), I(0), diagnostic.v); break;
    case OP_MOONCROSS: time = swe_mooncross(F(0), F(1), I(0), diagnostic.v); break;
    default: time = swe_mooncross_ut(F(0), F(1), I(0), diagnostic.v); break;
    }
    add_float(&result, time); TEXT(diagnostic); break;
}
case OP_MOONCROSS_NODE: case OP_MOONCROSS_NODE_UT: {
    SHAPE(1, 1, 0); DBUF(out, 3); TBUF(diagnostic, 256);
    if (request->op == OP_MOONCROSS_NODE) out.v[0] = swe_mooncross_node(F(0), I(0), &out.v[1], &out.v[2], diagnostic.v);
    else out.v[0] = swe_mooncross_node_ut(F(0), I(0), &out.v[1], &out.v[2], diagnostic.v);
    DOUBLES(out); TEXT(diagnostic); break;
}
case OP_HELIO_CROSS: case OP_HELIO_CROSS_UT: {
    SHAPE(3, 2, 0); DBUF(out, 1); TBUF(diagnostic, 256);
    if (request->op == OP_HELIO_CROSS) result.code = swe_helio_cross(I(0), F(0), F(1), I(1), I(2), out.v, diagnostic.v);
    else result.code = swe_helio_cross_ut(I(0), F(0), F(1), I(1), I(2), out.v, diagnostic.v);
    DOUBLES(out); TEXT(diagnostic); break;
}
