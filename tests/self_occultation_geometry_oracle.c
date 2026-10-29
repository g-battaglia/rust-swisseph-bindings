/* Public-header witness for the pinned engine's undefined self geometry.
 * Print only status/count evidence, never numerical geography/reference data.
 */
#include <math.h>
#include <stdio.h>
#include "swephexp.h"

int main(void) {
  double geography[10] = {0};
  double attributes[20] = {0};
  char diagnostic[256] = {0};
  int32 status = swe_lun_occult_where(2451545.0, SE_MOON, NULL, SEFLG_MOSEPH,
                                    geography, attributes, diagnostic);
  size_t nonfinite = 0;
  for (size_t i = 0; i < 10; ++i) {
    if (!isfinite(geography[i])) ++nonfinite;
  }
  printf("status=%d; nonfinite_geography=%zu; buffers=10/20\n", (int)status, nonfinite);
  swe_close();
  return status < 0 || nonfinite == 0;
}
