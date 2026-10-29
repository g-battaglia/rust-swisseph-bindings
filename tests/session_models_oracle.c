/* Public-ABI witness: original-precession sidereal options modify native model
 * state beyond the stored sidereal mode. Report counts only, never vectors. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static void defaults(int reset_models) {
    char models[32] = {0};
    if (reset_models) swe_set_astro_models(models, 0);
    swe_set_topo(0, 0, 0);
    swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY, 0, 0);
    swe_set_tid_acc(SE_TIDAL_AUTOMATIC);
    swe_set_delta_t_userdef(SE_DELTAT_AUTOMATIC);
    swe_set_lapse_rate(0.0065);
}

int main(void) {
    double before[6] = {0}, polluted[6] = {0}, restored[6] = {0};
    char diagnostic[256] = {0};
    const int32 flags = SEFLG_MOSEPH | SEFLG_SPEED;
    swe_set_ephe_path("/nonexistent/owned-session-model-witness");
    swe_set_jpl_file("");
    defaults(0);
    int32 first = swe_calc(2415020.0, SE_MOON, flags, before, diagnostic);
    swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY | SE_SIDBIT_PREC_ORIG, 0, 0);
    defaults(0);
    int32 second = swe_calc(2415020.0, SE_MOON, flags, polluted, diagnostic);
    defaults(1);
    int32 third = swe_calc(2415020.0, SE_MOON, flags, restored, diagnostic);
    unsigned changed = 0;
    for (unsigned i = 0; i < 6; ++i) {
        uint64_t a = 0, b = 0;
        memcpy(&a, &before[i], 8); memcpy(&b, &polluted[i], 8);
        changed += a != b;
    }
    int exact = first >= 0 && first == second && first == third
        && memcmp(before, restored, sizeof(before)) == 0;
    printf("changed_words=%u; defaults_restore_exact=%d\n", changed, exact);
    swe_close();
    return changed > 0 && exact ? 0 : 1;
}
