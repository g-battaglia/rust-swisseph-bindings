/* Public-ABI capacity probe for UTC diagnostics at the safe binding domain.
 * Only bounded seconds reach native C; hazardous seconds belong to Rust-only
 * rejection tests. Values and diagnostic text remain ephemeral. This checks
 * capacity/status, not astronomical accuracy or native implementation bodies. */
#include "swephexp.h"
#include <limits.h>
#include <math.h>
#include <stdio.h>

struct input {
    int year, month, day, hour, minute;
    double second;
    int error;
};

int main(void) {
    const struct input inputs[] = {
        {2000, 1, 1, 12, 0, 0.0, 0},
        {2000, 1, 1, 12, 0, -0.0, 0},
        {2000, 1, 1, 12, 0, 59.999999, 0},
        {2000, 13, 1, 12, 0, 0.0, 1},
        {2000, 2, 30, 12, 0, 0.0, 1},
        {2000, 1, 1, 24, 0, 0.0, 1},
        {2000, 1, 1, 12, 60, 0.0, 1},
        {2000, 1, 1, INT_MIN, INT_MIN, 60.5, 1},
        {2000, 1, 1, INT_MAX, INT_MAX, 60.5, 1},
        {2000, 1, 1, INT_MIN, INT_MAX, 60.999999, 1},
        {2000, 1, 1, INT_MAX, INT_MIN, 60.999999, 1},
        {2016, 6, 15, 23, 59, 60.0, 1},
        {2016, 6, 15, 23, 59, 60.999999, 1},
        {1960, 1, 1, 12, 0, 60.5, 1},
    };
    unsigned checks = 0;
    for (int cal = SE_JUL_CAL; cal <= SE_GREG_CAL; ++cal) {
        for (unsigned index = 0; index < sizeof inputs / sizeof inputs[0]; ++index) {
            const struct input *in = &inputs[index];
            double days[2] = {0};
            char diagnostic[1024] = {0};
            int32 status = swe_utc_to_jd(in->year, in->month, in->day,
                                        in->hour, in->minute, in->second, cal,
                                        days, diagnostic);
            if ((status < 0) != in->error) {
                fputs("FAIL UTC diagnostic probe: status classification\n", stderr);
                return 1;
            }
            for (unsigned byte = 256; byte < sizeof diagnostic; ++byte) {
                if (diagnostic[byte] != 0) {
                    fputs("FAIL UTC diagnostic probe: 256-byte capacity exceeded\n", stderr);
                    return 1;
                }
            }
            if (status >= 0 && (!isfinite(days[0]) || !isfinite(days[1]))) {
                fputs("FAIL UTC diagnostic probe: non-finite ordinary result\n", stderr);
                return 1;
            }
            ++checks;
        }
    }
    swe_close();
    printf("PASS UTC diagnostic capacity: %u bounded public-ABI calls\n", checks);
    return 0;
}
