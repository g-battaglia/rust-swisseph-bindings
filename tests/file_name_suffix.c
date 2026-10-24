/* Inputs and scalar counts only; filenames and oracle outputs stay in RAM. */
#include <limits.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "swephexp.h"
#include "sweph.h"
#include "swephlib.h"
#include "file_name_suffix.h"

#define REQUIRE(condition) do { \
  if (!(condition)) { \
    fprintf(stderr, "filename control failed at line %d\n", __LINE__); \
    exit(1); \
  } \
} while (0)

extern void oracle_swi_gen_filename(double, int, char *);

enum { GUARD_BYTES = 16, PREFIX_LENGTHS = 32 };
struct guarded_name {
  unsigned char before[GUARD_BYTES];
  char text[AS_MAXCH];
  unsigned char after[GUARD_BYTES];
};
static unsigned int suffix_cases, suffix_negative_controls, native_cases;

static void require_guards(const struct guarded_name *value)
{
  for (size_t index = 0; index < GUARD_BYTES; index++) {
    REQUIRE(value->before[index] == 0xa5);
    REQUIRE(value->after[index] == 0xa5);
  }
}

static void suffix_case(unsigned int century, size_t prefix_length)
{
  struct guarded_name actual, expected;
  REQUIRE(century <= INT_MAX && prefix_length < PREFIX_LENGTHS);
  memset(&actual, 0xa5, sizeof(actual));
  for (size_t index = 0; index < prefix_length; index++)
    actual.text[index] = (char)('A' + index % 26);
  actual.text[prefix_length] = '\0';
  memcpy(&expected, &actual, sizeof(expected));

  /* The complete independent formatter writes within the declared buffer.
   * A width of two is a minimum, not a restriction to two decimal digits. */
  int written = snprintf(expected.text + prefix_length,
                         sizeof(expected.text) - prefix_length,
                         "%02u.%s", century, SE_FILE_SUFFIX);
  REQUIRE(written > 0);
  REQUIRE((size_t)written < sizeof(expected.text) - prefix_length);
  swe_append_file_century(actual.text + prefix_length, century);
  REQUIRE(memcmp(&actual, &expected, sizeof(actual)) == 0);
  require_guards(&actual);
  size_t suffix_start = prefix_length + (size_t)written -
                        (sizeof(SE_FILE_SUFFIX) - 1);
  REQUIRE(memcmp(actual.text + suffix_start, SE_FILE_SUFFIX,
                 sizeof(SE_FILE_SUFFIX)) == 0);
  for (size_t index = prefix_length + (size_t)written + 1;
       index < sizeof(actual.text); index++)
    REQUIRE((unsigned char)actual.text[index] == 0xa5);

  /* A changed first digit must invalidate the full-buffer comparison. */
  expected.text[prefix_length] ^= 1;
  REQUIRE(memcmp(&actual, &expected, sizeof(actual)) != 0);
  suffix_negative_controls++;
  suffix_cases++;
}

static void suffix_boundaries(void)
{
  for (unsigned int century = 0; century < 100; century++)
    for (size_t prefix = 0; prefix < PREFIX_LENGTHS; prefix++)
      suffix_case(century, prefix);

  /* Every decimal-width transition and its adjacent values through INT_MAX.
   * Division in this fixture is not used to construct the expected digits. */
  for (unsigned int power = 10; power <= INT_MAX;) {
    for (size_t prefix = 0; prefix < PREFIX_LENGTHS; prefix++) {
      suffix_case(power - 1, prefix);
      suffix_case(power, prefix);
      suffix_case(power + 1, prefix);
    }
    if (power > (unsigned int)INT_MAX / 10)
      break;
    power *= 10;
  }
  for (unsigned int distance = 0; distance < 3; distance++)
    for (size_t prefix = 0; prefix < PREFIX_LENGTHS; prefix++)
      suffix_case((unsigned int)INT_MAX - distance, prefix);

  /* Deterministic unsigned arithmetic supplies extra input coverage only.
   * snprintf remains the independent expected-output implementation. */
  uint32_t state = UINT32_C(0x9e3779b9);
  for (unsigned int index = 0; index < 65536; index++) {
    state = state * UINT32_C(1664525) + UINT32_C(1013904223);
    unsigned int century = (unsigned int)(state & (uint32_t)INT_MAX);
    suffix_case(century, (state >> 27) % PREFIX_LENGTHS);
  }
}

static void native_case(double day, int body)
{
  struct guarded_name actual, expected;
  memset(&actual, 0xa5, sizeof(actual));
  memcpy(&expected, &actual, sizeof(expected));
  oracle_swi_gen_filename(day, body, expected.text);
  swi_gen_filename(day, body, actual.text);
  REQUIRE(memcmp(&actual, &expected, sizeof(actual)) == 0);
  require_guards(&actual);
  const char *terminator = memchr(actual.text, '\0', sizeof(actual.text));
  REQUIRE(terminator != NULL);
  size_t length = (size_t)(terminator - actual.text);
  for (size_t index = length + 1; index < sizeof(actual.text); index++)
    REQUIRE((unsigned char)actual.text[index] == 0xa5);
  native_cases++;
}

static void native_day_boundary(double day)
{
  static const int bodies[] = {
    SEI_EMB, SEI_MOON, SEI_MERCURY, SEI_VENUS, SEI_MARS, SEI_JUPITER,
    SEI_SATURN, SEI_URANUS, SEI_NEPTUNE, SEI_PLUTO, SEI_SUNBARY,
    SEI_CHIRON, SEI_PHOLUS, SEI_CERES, SEI_PALLAS, SEI_JUNO, SEI_VESTA,
    SE_PLMOON_OFFSET, SE_PLMOON_OFFSET + 1, SE_AST_OFFSET - 1,
    SE_AST_OFFSET, SE_AST_OFFSET + 1, SE_AST_OFFSET + 999,
    SE_AST_OFFSET + 1000, SE_AST_OFFSET + 99999, SE_AST_OFFSET + 100000,
    INT_MAX
  };
  double days[] = {nextafter(day, -INFINITY), day, nextafter(day, INFINITY)};
  for (size_t boundary = 0; boundary < sizeof(days) / sizeof(days[0]); boundary++)
    for (size_t body = 0; body < sizeof(bodies) / sizeof(bodies[0]); body++)
      native_case(days[boundary], bodies[body]);
}

static void native_date_boundaries(void)
{
  /* These are filename-only input dates. No ephemeris data or calculated
   * reference positions are generated, stored or needed by this control. */
  static const double days[] = {
    J2000, MOSHPLEPH_START, MOSHPLEPH_END,
    /* Legacy DE406 boundaries are retained as input dates. Their historical
     * JPLEPH_START/END macros are inside the pinned header's disabled block. */
    625307.5, 2816848.5,
    JPL_DE431_START, JPL_DE431_END, 2305447.5
  };
  for (size_t index = 0; index < sizeof(days) / sizeof(days[0]); index++)
    native_day_boundary(days[index]);

  /* Century division and six-century floor boundaries, including negative
   * years, year zero, and file-century values wider than two digits. */
  static const int years[] = {
    -13200, -13199, -10201, -10200, -10199, -10001, -10000, -9999,
    -601, -600, -599, -101, -100, -99, -1, 0, 1, 99, 100, 101,
    599, 600, 601, 1599, 1600, 1601, 9999, 10000, 10001,
    10199, 10200, 10201, 16799, 16800, 16801
  };
  for (size_t index = 0; index < sizeof(years) / sizeof(years[0]); index++) {
    int calendar = years[index] >= 1600 ? SE_GREG_CAL : SE_JUL_CAL;
    double day = swe_julday(years[index], 1, 1, 0.0, calendar);
    native_day_boundary(day);
  }
}

int main(void)
{
  suffix_boundaries();
  native_date_boundaries();
  printf("PASS filename control: %u suffix cases, %u negative controls, %u native date/body comparisons\n",
         suffix_cases, suffix_negative_controls, native_cases);
  return 0;
}
