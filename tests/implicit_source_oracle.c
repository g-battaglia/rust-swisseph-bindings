/* Native implicit-source result; binary stdout is consumed only in RAM. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static int emit(double jd, int32 body, int32 flags) {
  double words[6] = {0}; char diagnostic[256] = {0};
  int32 status = swe_calc_ut(jd, body, flags, words, diagnostic);
  fwrite(&status, sizeof(status), 1, stdout);
  fwrite(words, sizeof(double), 6, stdout);
  return status < 0;
}
int main(int argc, char **argv) {
  if (argc == 3 && !strcmp(argv[2], "sequence")) {
    double days[] = {2451545.0, 1700000.5};
    int32 bodies[] = {SE_SUN, SE_MOON, SE_MARS};
    int32 masks[] = {4, 260, 2308, 4356, 8452};
    int32 nutation[] = {0, 4, 260};
    swe_set_ephe_path(argv[1]);
    for (int day=0; day<2; ++day) {
      for (int body=0; body<3; ++body)
        for (int mask=0; mask<5; ++mask)
          if (emit(days[day], bodies[body], masks[mask])) return 4;
      for (int mask=0; mask<3; ++mask)
        if (emit(days[day], SE_ECL_NUT, nutation[mask])) return 4;
    }
  } else {
    if (argc != 5) return 2;
    double jd = strtod(argv[2], NULL), words[6] = {0};
    int32 body = (int32)strtol(argv[3], NULL, 10);
    int32 flags = (int32)strtol(argv[4], NULL, 10);
    char diagnostic[256] = {0};
    swe_set_ephe_path(argv[1]);
    if (swe_calc_ut(jd, SE_MARS, SEFLG_MOSEPH | SEFLG_SPEED | SEFLG_RADIANS,
                    words, diagnostic) < 0) return 4;
    if (emit(jd, body, flags)) return 4;
  }
  swe_close();
  return ferror(stdout) ? 3 : 0;
}
