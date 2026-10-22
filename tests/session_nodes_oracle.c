/* Public-header UT nodes/apsides output, retained only by the parent pipe. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
  if (argc != 7) return 2;
  double values[4][6] = {{0}};
  char diagnostic[256] = {0};
  int profile = (int)strtol(argv[6], NULL, 10);
  swe_set_ephe_path(argv[1]);
  swe_set_sid_mode((profile & 1) ? SE_SIDM_LAHIRI : SE_SIDM_FAGAN_BRADLEY, 0, 0);
  swe_set_topo((profile & 2) ? 2.35 : 0, (profile & 2) ? 48.86 : 0,
               (profile & 2) ? 35 : 0);
  int32 status = swe_nod_aps_ut(
      strtod(argv[2], NULL), (int32)strtol(argv[3], NULL, 10),
      (int32)strtol(argv[4], NULL, 10), (int32)strtol(argv[5], NULL, 10),
      values[0], values[1], values[2], values[3], diagnostic);
  uint32_t length = (uint32_t)strlen(diagnostic);
  fwrite(&status, sizeof(status), 1, stdout);
  fwrite(&length, sizeof(length), 1, stdout);
  fwrite(diagnostic, 1, length, stdout);
  fwrite(values, sizeof(double), 24, stdout);
  swe_close();
  return 0;
}
