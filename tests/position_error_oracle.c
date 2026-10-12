/* Independent C failure metadata; stdout is consumed only in memory. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
  if (argc != 3) return 2;
  double words[6] = {0}; char diagnostic[256] = {0};
  swe_set_ephe_path("/tmp/swiss-position-error-missing-data");
  int32 flags = (int32)strtol(argv[2], NULL, 10);
  int32 status = !strcmp(argv[1], "calc")
    ? swe_calc(2451545.0, 999999, flags, words, diagnostic)
    : swe_calc_ut(2451545.0, 999999, flags, words, diagnostic);
  uint32_t length = (uint32_t)strlen(diagnostic);
  fwrite(&status, sizeof(status), 1, stdout);
  fwrite(&length, sizeof(length), 1, stdout);
  fwrite(diagnostic, 1, length, stdout);
  swe_close(); return ferror(stdout) ? 3 : 0;
}
