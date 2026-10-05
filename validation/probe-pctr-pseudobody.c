/* Owned public-ABI-only repeatability probe for the ECL_NUT planet-centric
 * request. Links the exact Cargo-selected
 * archive; argv[1] is selected local ephemeris data. Configures once, then
 * issues six identical native calls with initialized buffers. All comparison
 * happens in memory; the report contains only repeatability counts, never
 * numerical vectors, bits, diagnostics or residuals. No native implementation
 * is inspected, copied or modified, and no production source changes. */
#include "swephexp.h"
#include <math.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) { fputs("requires selected local ephemeris directory\n", stderr); return 2; }
    swe_set_ephe_path(argv[1]);
    swe_set_jpl_file("__parity_missing_jpl__.eph");
    swe_set_topo(-93.052, 40.040000000000006, 2467.0);
    swe_set_sid_mode(17, 0, 0);
    swe_set_tid_acc(SE_TIDAL_AUTOMATIC);
    swe_set_delta_t_userdef(SE_DELTAT_AUTOMATIC);
    swe_set_lapse_rate(0.0065);
    double first[6] = {0};
    char diagnostic[256] = {0};
    int32 first_code = swe_calc_pctr(2443912.658, SE_ECL_NUT, SE_MARS,
                                     SEFLG_SPEED | SEFLG_TOPOCTR, first, diagnostic);
    unsigned differing = 0, nonfinite = 0, errors = 0;
    for (unsigned index = 1; index < 6; ++index) {
        double current[6] = {0};
        char serr[256] = {0};
        int32 code = swe_calc_pctr(2443912.658, SE_ECL_NUT, SE_MARS,
                                   SEFLG_SPEED | SEFLG_TOPOCTR, current, serr);
        int finite = 1;
        for (unsigned slot = 0; slot < 6; ++slot) finite &= isfinite(current[slot]);
        if (!finite) ++nonfinite;
        if (code < 0) ++errors;
        if (code != first_code || memcmp(current, first, sizeof first)) ++differing;
    }
    swe_close();
    printf("pctr pseudo-body public-C probe: repeats=5 differing=%u nonfinite=%u native-errors=%u\n", differing, nonfinite, errors);
    return differing || nonfinite ? 1 : 0;
}
