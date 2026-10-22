/* Direct public-header UT phenomena result, retained only by the parent pipe. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
  if (argc != 5) return 2;
  double values[20] = {0};
  char diagnostic[256] = {0};
  swe_set_ephe_path(argv[1]);
  int32 status = swe_pheno_ut(strtod(argv[2], NULL), (int32)strtol(argv[3], NULL, 10),
                             (int32)strtol(argv[4], NULL, 10), values, diagnostic);
  uint32_t length = (uint32_t)strlen(diagnostic);
  fwrite(&status, sizeof(status), 1, stdout);
  fwrite(&length, sizeof(length), 1, stdout);
  fwrite(diagnostic, 1, length, stdout);
  fwrite(values, sizeof(double), 20, stdout);
  swe_close();
  return 0;
}
