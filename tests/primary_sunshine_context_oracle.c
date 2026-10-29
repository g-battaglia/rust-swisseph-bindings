/* Public-header witness; report only status and a history-dependence count. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
int main(void) {
    const double dates[2] = {2451716.5, 2451900.5};
    double position[2]; int32 statuses[2];
    swe_set_ephe_path("/nonexistent/primary-sunshine-context");
    for (int i = 0; i < 2; ++i) {
        double cusps[13] = {0}, angles[10] = {0};
        double rates[13] = {0}, angle_rates[10] = {0};
        double point[2] = {80, 0}; char text[256] = {0};
        statuses[i] = swe_houses_ex2(dates[i], SEFLG_MOSEPH, 45, 0, 'I',
            cusps, angles, rates, angle_rates, text);
        position[i] = swe_house_pos(100, 45, 23.44, 'I', point, text);
    }
    uint64_t first, second;
    memcpy(&first, &position[0], sizeof first); memcpy(&second, &position[1], sizeof second);
    printf("history_dependent=%d statuses=%d,%d\n", first != second, statuses[0], statuses[1]);
    swe_close();
    return 0;
}
