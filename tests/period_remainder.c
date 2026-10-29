/* Independent exact-bit controls; no calculated values are printed or saved. */
#pragma STDC FENV_ACCESS ON
#include <fenv.h>
#include <inttypes.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "swephexp.h"
#include "sweph.h"
#include "exact_period_remainder.h"

extern double oracle_fmod(double, double);
extern double oracle_degnorm(double);
extern double oracle_radnorm(double);

static double upward_reciprocal(double period)
{
  return period == 360.0 ? @DEGREE_RECIPROCAL@ : @RADIAN_RECIPROCAL@;
}

static uint64_t checks, integer_checks, nonfinite_checks;
static uint64_t inexact_differences, quotient_corrections;

static uint64_t bits(double x)
{
  uint64_t representation;
  memcpy(&representation, &x, sizeof representation);
  return representation;
}

static double value(uint64_t representation)
{
  double x;
  memcpy(&x, &representation, sizeof x);
  return x;
}

static double normalizer(double x, double p)
{
  double y = swe_exact_period_remainder(x, p, upward_reciprocal(p));
  if (fabs(y) < 1e-13) y = 0;
  if (y < 0.0) y += p;
  return y;
}

/* Independent Euclidean integer arithmetic: scale both significands by the
 * period's binary exponent. The numerator has at most 103 bits. Conversion of
 * the remainder back to double is exact and never needs FMA or division. */
static double integer_oracle(double x, double p)
{
  uint64_t magnitude_bits = bits(fabs(x));
  uint64_t period_bits = bits(p);
  int magnitude_exponent = (int)(magnitude_bits >> 52);
  int period_exponent = (int)(period_bits >> 52);
  uint64_t magnitude_significand = (magnitude_bits & UINT64_C(0xfffffffffffff)) | (UINT64_C(1) << 52);
  uint64_t period_significand = (period_bits & UINT64_C(0xfffffffffffff)) | (UINT64_C(1) << 52);
  __uint128_t numerator = (__uint128_t)magnitude_significand << (magnitude_exponent - period_exponent);
  uint64_t remainder = (uint64_t)(numerator % period_significand);
  if ((uint64_t)trunc(fabs(x) * upward_reciprocal(p)) > (uint64_t)(numerator / period_significand)) {
    quotient_corrections++;
  }
  return copysign(ldexp((double)remainder, period_exponent - 1023 - 52), x);
}

static int check(double x, double p, int period_index, int mode)
{
  double actual = swe_exact_period_remainder(x, p, upward_reciprocal(p));
  double expected = oracle_fmod(x, p);
  checks++;
  if (bits(actual)!=bits(expected)) {
    fprintf(stderr,"primitive mismatch: input_bits=%016" PRIx64 " period=%d mode=%d\n",bits(x),period_index,mode);
    return 0;
  }
  if (!isfinite(x)) nonfinite_checks++;
  if (isfinite(x) && fabs(x)>=p && fabs(x)<=p*0x1p50) {
    integer_checks++;
    if (bits(actual)!=bits(integer_oracle(x,p))) {
      fprintf(stderr,"integer-oracle mismatch: input_bits=%016" PRIx64 " period=%d mode=%d\n",bits(x),period_index,mode);
      return 0;
    }
  }
  expected=period_index ? oracle_radnorm(x) : oracle_degnorm(x);
  actual=normalizer(x,p);
  checks++;
  if (bits(actual)!=bits(expected)) {
    fprintf(stderr,"normalizer mismatch: input_bits=%016" PRIx64 " period=%d mode=%d\n",bits(x),period_index,mode);
    return 0;
  }
  return 1;
}

static int signs(double x,double p,int period,int mode)
{
  return check(x,p,period,mode) && check(-x,p,period,mode);
}

int main(void)
{
  const int modes[]={FE_TONEAREST,FE_DOWNWARD,FE_UPWARD,FE_TOWARDZERO};
  const double periods[]={360.0,TWOPI};
  const uint64_t special[]={0,1,UINT64_C(0xfffffffffffff),UINT64_C(0x10000000000000),
    UINT64_C(0x7fefffffffffffff),UINT64_C(0x7ff0000000000000),
    UINT64_C(0x7ff8000000000001),UINT64_C(0x7ff0000000000001),UINT64_C(0x7fffffffffffffff)};
  for (unsigned mode=0;mode<4;mode++) {
    if (fesetround(modes[mode])) return 2;
    for (int period=0;period<2;period++) {
      double p=periods[period];
      for (unsigned i=0;i<sizeof special/sizeof *special;i++)
        if (!signs(value(special[i]),p,period,mode)) return 1;
      /* Deterministic coverage of every finite exponent and every mantissa
       * bit, plus endpoint, midpoint and pseudorandom significands. */
      uint64_t state = UINT64_C(0xc61d9e8f1234abcd);
      for (uint64_t exponent=0;exponent<2047;exponent++) {
        for (unsigned mantissa=0;mantissa<64;mantissa++) {
          state ^= state << 13;
          state ^= state >> 7;
          state ^= state << 17;
          uint64_t m=mantissa<52 ? UINT64_C(1)<<mantissa : state;
          if (mantissa==52) m=0;
          if (mantissa==53) m=UINT64_C(0xfffffffffffff);
          if (mantissa==54) m=UINT64_C(0x7ffffffffffff);
          if (mantissa==55) m=UINT64_C(0x8000000000000);
          if (!signs(value((exponent<<52)|(m & UINT64_C(0xfffffffffffff))),p,period,mode)) return 1;
        }
      }
      /* Neighboring representable inputs at multiples, powers of two,
       * the proven-domain endpoint and the native 1e-13 zero threshold. */
      for (unsigned i=0;i<4096;i++) {
        double x=(double)i*p;
        if (!signs(x,p,period,mode) || !signs(nextafter(x,INFINITY),p,period,mode)
            || !signs(nextafter(x,-INFINITY),p,period,mode)) return 1;
      }
      for (int exponent=-1074;exponent<=1023;exponent++) {
        double x=ldexp(1.0,exponent);
        if (!signs(x,p,period,mode) || !signs(nextafter(x,INFINITY),p,period,mode)
            || !signs(nextafter(x,-INFINITY),p,period,mode)) return 1;
      }
      for (int exponent=0;exponent<=50;exponent++) {
        double x=ldexp(p,exponent);
        if (!signs(x,p,period,mode) || !signs(nextafter(x,INFINITY),p,period,mode)
            || !signs(nextafter(x,-INFINITY),p,period,mode)) return 1;
      }
      const double endpoints[]={p,p*0x1p50,1e-13};
      for (unsigned i=0;i<sizeof endpoints/sizeof *endpoints;i++) {
        double x=endpoints[i];
        if (!signs(x,p,period,mode) || !signs(nextafter(x,INFINITY),p,period,mode)
            || !signs(nextafter(x,-INFINITY),p,period,mode)) return 1;
      }
      double x = p + 1.0;
      feclearexcept(FE_ALL_EXCEPT);
      volatile double actual = swe_exact_period_remainder(x, p, upward_reciprocal(p));
      int candidate_flags = fetestexcept(FE_ALL_EXCEPT);
      feclearexcept(FE_ALL_EXCEPT);
      volatile double expected = oracle_fmod(x, p);
      int reference_flags = fetestexcept(FE_ALL_EXCEPT);
      (void)actual;
      (void)expected;
      if ((candidate_flags ^ reference_flags) & ~FE_INEXACT) return 3;
      if ((candidate_flags ^ reference_flags) & FE_INEXACT) inexact_differences++;
    }
  }
  fesetround(FE_TONEAREST);
  if (!quotient_corrections) return 4;
  printf("PASS exact remainder: checks=%" PRIu64 " integer_checks=%" PRIu64 " nonfinite=%" PRIu64 " modes=4 inexact_differences=%" PRIu64 " quotient_corrections=%" PRIu64 "\n",checks,integer_checks,nonfinite_checks,inexact_differences,quotient_corrections);
  return 0;
}
