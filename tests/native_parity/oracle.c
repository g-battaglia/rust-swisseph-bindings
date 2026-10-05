/* Independent public-ABI caller. Only swephexp.h declares native functions.
 * No Rust validation/packing helper or native astronomical implementation is
 * copied here. Output allocations have exactly the declared ABI capacities;
 * guards check writes beyond them before any bytes are transported. */
#include "swephexp.h"
#include "operations.h"
#include "wire.h"

#define CANARY 0xa7
#define DBUF(name, count) struct { unsigned char before[16]; double v[count]; unsigned char after[16]; } name = {{0}, {0}, {0}}; memset(name.before, CANARY, 16); memset(name.after, CANARY, 16)
#define TBUF(name, count) struct { unsigned char before[16]; char v[count]; unsigned char after[16]; } name = {{0}, {0}, {0}}; memset(name.before, CANARY, 16); memset(name.after, CANARY, 16)
#define CHECK(name) check_guards(name.before, name.after)
#define DOUBLES(name) do { CHECK(name); add_floats(&result, name.v, sizeof(name.v) / sizeof(double)); } while (0)
#define TEXT(name) do { CHECK(name); add_bounded_text(&result, name.v, sizeof(name.v)); } while (0)
#define I(n) request->ints[n]
#define F(n) request->floats[n]
#define T(n) (request->present[n] ? request->texts[n] : NULL)
#define SHAPE(i, f, t) require(request->ni == (i) && request->nf == (f) && request->nt == (t), "argument shape")

/* Buffer guards are synthetic bytes, never reference numerical outputs. */
static void check_guards(const unsigned char *before, const unsigned char *after) {
    for (unsigned i = 0; i < 16; ++i) require(before[i] == CANARY && after[i] == CANARY, "native output buffer overrun");
}

/* Model the safe setter's promised effective state using only public ABI:
 * initialize defaults before a first tidal/Delta-T write, not on every call.
 * swe_close and explicit sidereal writes update this worker-local history. */
static int initialized_settings = 0;
static void initialize_settings(void) {
    if (!initialized_settings) { swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY, 0, 0); initialized_settings = 1; }
}

/* Copy an explicitly validated UTF-8 query into writable caller storage.
 * Count is the real public-ABI capacity, including a required terminator. */
static void copy_query(char *destination, size_t count, const char *query) {
    require(query != NULL && strlen(query) < count, "query capacity/null");
    memcpy(destination, query, strlen(query) + 1);
}

/* A native status and its complete diagnostic are retained independently of
 * whether the safe API exposes a status field, an enum or an error. */
static Result dispatch(const Request *request) {
    Result result = {0}; result.id = request->id; result.op = request->op;
    switch (request->op) {
    case 1000: {
        SHAPE(0, 0, 0); TBUF(version, 256); swe_version(version.v);
        add_int(&result, sizeof(int32)); add_int(&result, sizeof(double)); add_int(&result, sizeof(char)); TEXT(version); break;
    }
    case 1001: {
        SHAPE(0, 0, 0);
#include "constants.inc"
        break;
    }
    case OP_VERSION: { SHAPE(0, 0, 0); TBUF(out, 256); swe_version(out.v); TEXT(out); break; }
    case OP_LIBRARY_PATH: { SHAPE(0, 0, 0); TBUF(out, 257); swe_get_library_path(out.v); TEXT(out); break; }
    case OP_GET_PLANET_NAME: { SHAPE(1, 0, 0); TBUF(out, 256); swe_get_planet_name(I(0), out.v); TEXT(out); break; }
    case OP_HOUSE_NAME: {
        SHAPE(1, 0, 0); const char *name = swe_house_name(I(0)); require(name != NULL, "null house name"); add_text(&result, name, strlen(name)); break;
    }
    case OP_GET_AYANAMSA_NAME: {
        SHAPE(1, 0, 0); const char *name = swe_get_ayanamsa_name(I(0));
        if (!name) { result.code = -1; add_text(&result, "", 0); } else add_text(&result, name, strlen(name)); break;
    }
    case OP_JULDAY: { SHAPE(4, 1, 0); add_float(&result, swe_julday(I(0), I(1), I(2), F(0), I(3))); break; }
    case OP_REVJUL: {
        SHAPE(1, 1, 0); int y = 0, m = 0, d = 0; double hour = 0;
        swe_revjul(F(0), I(0), &y, &m, &d, &hour);
        add_int(&result, y); add_int(&result, m); add_int(&result, d); add_float(&result, hour); break;
    }
    case OP_UTC_TO_JD: {
        SHAPE(6, 1, 0); DBUF(out, 2); TBUF(diagnostic, 256);
        result.code = swe_utc_to_jd(I(0), I(1), I(2), I(3), I(4), F(0), I(5), out.v, diagnostic.v);
        DOUBLES(out); TEXT(diagnostic); break;
    }
    case OP_JDET_TO_UTC: case OP_JDUT1_TO_UTC: case OP_UTC_TIME_ZONE: {
        int32 y = 0, m = 0, d = 0, h = 0, minute = 0; double second = 0;
        if (request->op == OP_UTC_TIME_ZONE) { SHAPE(5, 2, 0); swe_utc_time_zone(I(0), I(1), I(2), I(3), I(4), F(0), F(1), &y, &m, &d, &h, &minute, &second); }
        else if (request->op == OP_JDET_TO_UTC) { SHAPE(1, 1, 0); swe_jdet_to_utc(F(0), I(0), &y, &m, &d, &h, &minute, &second); }
        else { SHAPE(1, 1, 0); swe_jdut1_to_utc(F(0), I(0), &y, &m, &d, &h, &minute, &second); }
        add_int(&result, y); add_int(&result, m); add_int(&result, d); add_int(&result, h); add_int(&result, minute); add_float(&result, second); break;
    }
    case OP_DATE_CONVERSION: {
        SHAPE(4, 1, 0); double jd = 0; result.code = swe_date_conversion(I(0), I(1), I(2), F(0), I(3) == SE_GREG_CAL ? 'g' : 'j', &jd);
        add_float(&result, jd); if (result.code < 0) add_text(&result, "", 0); break;
    }
    case OP_DAY_OF_WEEK: { SHAPE(0, 1, 0); add_int(&result, swe_day_of_week(F(0))); break; }
    case OP_DELTAT: { SHAPE(0, 1, 0); add_float(&result, swe_deltat(F(0))); break; }
    case OP_DELTAT_EX: {
        SHAPE(1, 1, 0); TBUF(diagnostic, 256); add_float(&result, swe_deltat_ex(F(0), I(0), diagnostic.v)); TEXT(diagnostic); break;
    }
    case OP_TIME_EQU: case OP_LMT_TO_LAT: case OP_LAT_TO_LMT: {
        TBUF(diagnostic, 256); double value = 0;
        if (request->op == OP_TIME_EQU) { SHAPE(0, 1, 0); result.code = swe_time_equ(F(0), &value, diagnostic.v); }
        else if (request->op == OP_LMT_TO_LAT) { SHAPE(0, 2, 0); result.code = swe_lmt_to_lat(F(0), F(1), &value, diagnostic.v); }
        else { SHAPE(0, 2, 0); result.code = swe_lat_to_lmt(F(0), F(1), &value, diagnostic.v); }
        add_float(&result, value); TEXT(diagnostic); break;
    }
    case OP_SIDTIME: { SHAPE(0, 1, 0); add_float(&result, swe_sidtime(F(0))); break; }
    case OP_SIDTIME0: { SHAPE(0, 3, 0); add_float(&result, swe_sidtime0(F(0), F(1), F(2))); break; }
    case OP_CALC: case OP_CALC_UT: case OP_CALC_PCTR: {
        DBUF(out, 6); TBUF(diagnostic, 256);
        if (request->op == OP_CALC_PCTR) { SHAPE(3, 1, 0); result.code = swe_calc_pctr(F(0), I(0), I(1), I(2), out.v, diagnostic.v); }
        else if (request->op == OP_CALC_UT) { SHAPE(2, 1, 0); result.code = swe_calc_ut(F(0), I(0), I(1), out.v, diagnostic.v); }
        else { SHAPE(2, 1, 0); result.code = swe_calc(F(0), I(0), I(1), out.v, diagnostic.v); }
        DOUBLES(out); TEXT(diagnostic); break;
    }
    case OP_SET_EPHE_PATH: { SHAPE(0, 0, 1); swe_set_ephe_path(T(0)); break; }
    case OP_SET_JPL_FILE: { SHAPE(0, 0, 1); require(T(0) != NULL, "null JPL name"); swe_set_jpl_file(T(0)); break; }
    case OP_SET_TOPO: { SHAPE(0, 3, 0); swe_set_topo(F(0), F(1), F(2)); break; }
    case OP_SET_SID_MODE: { SHAPE(1, 2, 0); swe_set_sid_mode(I(0), F(0), F(1)); initialized_settings = 1; break; }
    case OP_SET_TID_ACC: { SHAPE(0, 1, 0); initialize_settings(); swe_set_tid_acc(F(0)); break; }
    case OP_GET_TID_ACC: { SHAPE(0, 0, 0); add_float(&result, swe_get_tid_acc()); break; }
    case OP_SET_DELTA_T_USERDEF: { SHAPE(1, 1, 0); initialize_settings(); swe_set_delta_t_userdef(I(0) ? F(0) : SE_DELTAT_AUTOMATIC); break; }
    case OP_SET_LAPSE_RATE: { SHAPE(0, 1, 0); swe_set_lapse_rate(F(0)); break; }
    case OP_CLOSE: { SHAPE(0, 0, 0); swe_close(); initialized_settings = 0; break; }
    case OP_GET_AYANAMSA: { SHAPE(0, 1, 0); add_float(&result, swe_get_ayanamsa(F(0))); break; }
    case OP_GET_AYANAMSA_UT: { SHAPE(0, 1, 0); add_float(&result, swe_get_ayanamsa_ut(F(0))); break; }
    case OP_GET_AYANAMSA_EX: case OP_GET_AYANAMSA_EX_UT: {
        SHAPE(1, 1, 0); double value = 0; TBUF(diagnostic, 256);
        if (request->op == OP_GET_AYANAMSA_EX) result.code = swe_get_ayanamsa_ex(F(0), I(0), &value, diagnostic.v);
        else result.code = swe_get_ayanamsa_ex_ut(F(0), I(0), &value, diagnostic.v);
        add_float(&result, value); TEXT(diagnostic); break;
    }
    case OP_HOUSES: case OP_HOUSES_EX: case OP_HOUSES_ARMC: {
        DBUF(cusps, 13); DBUF(angles, 10);
        if (request->op == OP_HOUSES) { SHAPE(1, 3, 0); result.code = swe_houses(F(0), F(1), F(2), I(0), cusps.v, angles.v); }
        else if (request->op == OP_HOUSES_EX) { SHAPE(2, 3, 0); result.code = swe_houses_ex(F(0), I(0), F(1), F(2), I(1), cusps.v, angles.v); }
        else { SHAPE(1, 3, 0); result.code = swe_houses_armc(F(0), F(1), F(2), I(0), cusps.v, angles.v); }
        DOUBLES(cusps); DOUBLES(angles); if (result.code < 0) add_text(&result, "", 0); break;
    }
    case OP_HOUSES_EX2: case OP_HOUSES_ARMC_EX2: {
        DBUF(cusps, 13); DBUF(angles, 10); DBUF(speeds, 13); DBUF(angle_speeds, 10); TBUF(diagnostic, 256);
        if (request->op == OP_HOUSES_EX2) { SHAPE(2, 3, 0); result.code = swe_houses_ex2(F(0), I(0), F(1), F(2), I(1), cusps.v, angles.v, speeds.v, angle_speeds.v, diagnostic.v); }
        else { SHAPE(1, 4, 0); angles.v[9] = F(3); result.code = swe_houses_armc_ex2(F(0), F(1), F(2), I(0), cusps.v, angles.v, speeds.v, angle_speeds.v, diagnostic.v); }
        DOUBLES(cusps); DOUBLES(angles); DOUBLES(speeds); DOUBLES(angle_speeds); TEXT(diagnostic); break;
    }
    case OP_HOUSES_GAUQUELIN: {
        SHAPE(1, 3, 0); DBUF(cusps, 37); DBUF(angles, 10); DBUF(speeds, 37); DBUF(angle_speeds, 10); TBUF(diagnostic, 256);
        result.code = swe_houses_ex2(F(0), I(0), F(1), F(2), 'G', cusps.v, angles.v, speeds.v, angle_speeds.v, diagnostic.v);
        DOUBLES(cusps); DOUBLES(angles); DOUBLES(speeds); DOUBLES(angle_speeds); TEXT(diagnostic); break;
    }
    case OP_HOUSE_POS: {
        SHAPE(1, 5, 0); double point[2] = {F(3), F(4)}; TBUF(diagnostic, 256);
        add_float(&result, swe_house_pos(F(0), F(1), F(2), I(0), point, diagnostic.v)); TEXT(diagnostic); break;
    }
    case OP_GET_CURRENT_FILE_DATA: {
        SHAPE(1, 0, 0); double first = 0, last = 0; int denum = 0;
        const char *path = swe_get_current_file_data(I(0), &first, &last, &denum);
        add_int(&result, path != NULL);
        if (path) { add_int(&result, denum); add_float(&result, first); add_float(&result, last); add_text(&result, path, strlen(path)); } break;
    }
    case OP_COTRANS: { SHAPE(0, 4, 0); double input[3] = {F(0), F(1), F(2)}; DBUF(out, 3); swe_cotrans(input, out.v, F(3)); DOUBLES(out); break; }
    case OP_COTRANS_SP: { SHAPE(0, 7, 0); double input[6] = {F(0), F(1), F(2), F(3), F(4), F(5)}; DBUF(out, 6); swe_cotrans_sp(input, out.v, F(6)); DOUBLES(out); break; }
#define UNARY(op, symbol) case op: SHAPE(0, 1, 0); add_float(&result, symbol(F(0))); break
#define BINARY(op, symbol) case op: SHAPE(0, 2, 0); add_float(&result, symbol(F(0), F(1))); break
    UNARY(OP_DEGNORM, swe_degnorm); UNARY(OP_RADNORM, swe_radnorm);
    BINARY(OP_DIFDEGN, swe_difdegn); BINARY(OP_DIFDEG2N, swe_difdeg2n); BINARY(OP_DIFRAD2N, swe_difrad2n); BINARY(OP_DEG_MIDP, swe_deg_midp); BINARY(OP_RAD_MIDP, swe_rad_midp);
    case OP_D2L: { SHAPE(0, 1, 0); add_int(&result, swe_d2l(F(0))); break; }
    case OP_CSNORM: { SHAPE(1, 0, 0); add_int(&result, swe_csnorm(I(0))); break; }
    case OP_DIFCSN: { SHAPE(2, 0, 0); add_int(&result, swe_difcsn(I(0), I(1))); break; }
    case OP_DIFCS2N: { SHAPE(2, 0, 0); add_int(&result, swe_difcs2n(I(0), I(1))); break; }
    case OP_CSROUNDSEC: { SHAPE(1, 0, 0); add_int(&result, swe_csroundsec(I(0))); break; }
    case OP_CS2TIMESTR: { SHAPE(3, 0, 0); TBUF(out, 256); swe_cs2timestr(I(0), I(1), I(2), out.v); TEXT(out); break; }
    case OP_CS2LONLATSTR: { SHAPE(3, 0, 0); TBUF(out, 256); swe_cs2lonlatstr(I(0), (char)I(1), (char)I(2), out.v); TEXT(out); break; }
    case OP_CS2DEGSTR: { SHAPE(1, 0, 0); TBUF(out, 256); swe_cs2degstr(I(0), out.v); TEXT(out); break; }
    case OP_SPLIT_DEG: {
        SHAPE(1, 1, 0); int32 deg = 0, minute = 0, second = 0, sign = 0; double fraction = 0;
        swe_split_deg(F(0), I(0), &deg, &minute, &second, &fraction, &sign);
        add_int(&result, deg); add_int(&result, minute); add_int(&result, second); add_int(&result, sign); add_float(&result, fraction); break;
    }
#include "physical.inc"
#include "crossing.inc"
#include "eclipse.inc"
#include "heliacal.inc"
    default: fail("unimplemented operation");
    }
    return result;
}

/* EOF, errors and native termination stay observable to the supervising parent. */
int main(void) {
    Request request;
    while (read_request(&request)) { Result result = dispatch(&request); write_result(&result); }
    swe_close(); return 0;
}
