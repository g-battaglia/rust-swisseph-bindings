/* SPDX-License-Identifier: AGPL-3.0-only */
#ifndef SWE_EXACT_PERIOD_REMAINDER_H
#define SWE_EXACT_PERIOD_REMAINDER_H

#include <math.h>

/* Exact fmod for the positive normal periods 360.0 and TWOPI. Outside the
 * proven domain use libc, including its nonfinite result handling.
 *
 * Let m=floor(a/period), period <= a <= period*2^50. The admitted constant is the
 * successor of the nearest-even reciprocal. Integer generation proves its
 * relative excess is positive and below 3*2^-53. Hence exact a*reciprocal is
 * greater than a/period by less than 3/8; multiplication rounding adds
 * at most 1/4. In every rounding direction trunc therefore gives m or m+1.
 *
 * Writing period=P*2^e, the integer residual has at most 53 bits: FMA returns
 * exactly u or u-period and the correction is exact. Copying the input sign
 * preserves both zero signs. Only the two compile-admitted periods/constants
 * satisfy this private contract. Errno/fenv/trap side effects are not promised;
 * quotient multiplication may additionally raise FE_INEXACT. */
static inline double swe_exact_period_remainder(double x, double period,
                                                 double reciprocal)
{
  double a, q, r;
  if (!isfinite(x))
    return fmod(x, period);
  a = fabs(x);
  if (a < period)
    return x;
  if (!(a <= period * 0x1p50))
    return fmod(x, period);
  q = trunc(a * reciprocal);
  r = fma(-q, period, a);
  if (r < 0.0)
    r += period;
  return copysign(r, x);
}
#endif
