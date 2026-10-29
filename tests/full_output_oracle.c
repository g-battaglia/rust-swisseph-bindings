/* Independent public-header caller. Binary output is consumed only in memory. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
  if (argc != 5 && argc != 6) return 2;
  swe_set_ephe_path(argv[1]);
  const char *jpl = getenv("SWISSEPH_JPL_FILE");
  if (jpl) {
    const char *base = strrchr(jpl, '/');
    swe_set_jpl_file((char *)(base ? base + 1 : jpl));
  }
  double jd = strtod(argv[3], NULL);
  int32 flags = (int32)strtol(argv[4], NULL, 10), status;
  double geo[10] = {0}, attr[20] = {0}, values[50] = {0}, visibility[10] = {0};
  char diagnostic[256] = {0};
  uint32_t count;
  if (!strcmp(argv[2], "solar")) {
    status = swe_sol_eclipse_where(jd, flags, geo, attr, diagnostic);
    count = 30;
  } else if (!strcmp(argv[2], "occult")) {
    status = swe_lun_occult_where(jd, SE_SUN, NULL, flags, geo, attr, diagnostic);
    count = 30;
  } else if (!strcmp(argv[2], "heliacal")) {
    double site[3] = {0, 45, 0}, atm[4] = {1013.25, 15, 40, 0};
    double observer[6] = {36, 1, 0, 0, 0, 0};
    char object[256] = "Venus";
    status = swe_heliacal_pheno_ut(jd, site, atm, observer, object,
                                  SE_HELIACAL_RISING, flags, values, diagnostic);
    count = 50;
  } else if (!strcmp(argv[2], "heliacal-event")) {
    double site[3] = {0, 51.5, 0}, atm[4] = {0}, observer[6] = {0};
    char object[256] = "Venus";
    status = swe_heliacal_ut(jd, site, atm, observer, object,
                            SE_HELIACAL_RISING, flags, visibility, diagnostic);
    count = 10;
  } else if (!strcmp(argv[2], "vislim")) {
    double site[3] = {0, 51.5, 0}, atm[4] = {0}, observer[6] = {0};
    char object[256] = "Venus";
    if (argc == 6) {
      if (strlen(argv[5]) >= sizeof(object)) return 2;
      strcpy(object, argv[5]);
    }
    status = swe_vis_limit_mag(jd, site, atm, observer, object, flags,
                              visibility, diagnostic);
    count = 10;
  } else return 3;
  /* Below-horizon visibility is data, not an undefined failure buffer. */
  if (status < 0 && !(count == 10 && status == -2)) count = 0;
  uint32_t text_len = (uint32_t)strlen(diagnostic);
  fwrite(&status, sizeof(status), 1, stdout);
  fwrite(&count, sizeof(count), 1, stdout);
  if (count == 30) {
    fwrite(geo, sizeof(double), 10, stdout);
    fwrite(attr, sizeof(double), 20, stdout);
  } else if (count == 50) fwrite(values, sizeof(double), 50, stdout);
  else if (count == 10) fwrite(visibility, sizeof(double), 10, stdout);
  fwrite(&text_len, sizeof(text_len), 1, stdout);
  fwrite(diagnostic, 1, text_len, stdout);
  swe_close();
  return ferror(stdout) ? 4 : 0;
}
