/* Native admission cases, binary responses remain in the harness memory. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
  if (argc != 4) return 2;
  swe_set_ephe_path("/tmp/swiss-admission-missing-data");
  int32 flags = (int32)strtol(argv[3], NULL, 10), status;
  double values[30] = {0}; char diagnostic[256] = {0}, name[256] = {0};
  uint32_t count = 6; double scalar = 0;
  if (!strcmp(argv[1], "fixstar2") || !strcmp(argv[1], "fixstar2_ut")) {
    snprintf(name, sizeof(name), "%s", argv[2]);
    status = !strcmp(argv[1], "fixstar2")
      ? swe_fixstar2(name, 2451545.0, flags, values, diagnostic)
      : swe_fixstar2_ut(name, 2451545.0, flags, values, diagnostic);
  } else if (!strcmp(argv[1], "where")) {
    status = swe_lun_occult_where(strtod(argv[2], NULL), SE_SUN, NULL,
                                 flags, values, values + 10, diagnostic);
    count = 30;
  } else if (!strcmp(argv[1], "catalog-preflight")) {
    double magnitude = 0; snprintf(name, sizeof(name), "1");
    status = swe_fixstar_mag(name, &magnitude, diagnostic); count = 0;
  } else if (!strcmp(argv[1], "mooncross_ut")) {
    scalar = swe_mooncross_ut(90.0, 2460310.5, flags, diagnostic);
    status = 0; count = 1; values[0] = scalar;
  } else return 3;
  uint32_t length = (uint32_t)strlen(diagnostic), name_length = (uint32_t)strlen(name);
  fwrite(&status, sizeof(status), 1, stdout);
  fwrite(&count, sizeof(count), 1, stdout);
  fwrite(values, sizeof(double), count, stdout);
  fwrite(&length, sizeof(length), 1, stdout); fwrite(diagnostic, 1, length, stdout);
  fwrite(&name_length, sizeof(name_length), 1, stdout); fwrite(name, 1, name_length, stdout);
  swe_close(); return ferror(stdout) ? 4 : 0;
}
