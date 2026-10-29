/* Public-header calendar transport witness. Only inputs are enumerated here;
 * every numerical result is obtained from the linked native implementation.
 * Output stays in the test's pipe, never in committed reference vectors. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void output(const void *value, size_t bytes) {
    if (fwrite(value, 1, bytes, stdout) != bytes) exit(2);
}

int main(void) {
    const int32 years[] = {-10000, -4712, -1, 0, 1582, 1900, 2000, 2024, 10000};
    const int32 months[] = {-1, 1, 2, 10, 12, 13};
    const int32 days[] = {-1, 0, 1, 28, 29, 30, 31, 32};
    const double hours[] = {-1.5, -0.0, 0.0, 12.123456789, 24.0, 25.0};
    for (size_t y = 0; y < sizeof(years) / sizeof(years[0]); ++y)
    for (size_t m = 0; m < sizeof(months) / sizeof(months[0]); ++m)
    for (size_t d = 0; d < sizeof(days) / sizeof(days[0]); ++d)
    for (size_t h = 0; h < sizeof(hours) / sizeof(hours[0]); ++h)
    for (int32 cal = SE_JUL_CAL; cal <= SE_GREG_CAL; ++cal) {
        int32 year = years[y], month = months[m], day = days[d];
        double hour = hours[h];
        double jd = swe_julday(year, month, day, hour, cal), checked = 0;
        int32 status = swe_date_conversion(year, month, day, hour,
            cal == SE_JUL_CAL ? 'j' : 'g', &checked);
        int32 out_year = 0, out_month = 0, out_day = 0;
        double out_hour = 0;
        swe_revjul(checked, cal, &out_year, &out_month, &out_day, &out_hour);
        int32 weekday = swe_day_of_week(jd);
        output(&year, 4); output(&month, 4); output(&day, 4); output(&cal, 4);
        output(&hour, 8); output(&jd, 8); output(&checked, 8); output(&status, 4);
        output(&out_year, 4); output(&out_month, 4); output(&out_day, 4);
        output(&out_hour, 8); output(&weekday, 4);
    }
    return fflush(stdout) == 0 ? 0 : 2;
}
