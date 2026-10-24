/* SPDX-License-Identifier: AGPL-3.0-only */
#ifndef SWE_EXACT_PERIOD_REMAINDER_H
#define SWE_EXACT_PERIOD_REMAINDER_H

#include <math.h>

/* Exact fmod for the positive normal periods 360.0 and TWOPI. Outside the
 * proven domain use libc, including its nonfinite result handling.
 *
 * For a >= period, write period = P*2^e with integer P < 2^53. Every binary64
 * a is an integer multiple of 2^e. With a <= period*2^50, trunc(a/period)
 * is m or m+1, where m=floor(a/period); m is exactly representable. The
 * integer residual has at most 53 bits, so FMA returns exactly u or u-period.
 * The correction is exact too, in every IEEE rounding direction. Copying the
 * original sign preserves both zero signs. No reciprocal or split product
 * approximates the remainder. Division may additionally raise FE_INEXACT;
 * this helper does not promise libc errno/fenv/trap side-effect equivalence. */
static inline double swe_exact_period_remainder(double x, double period)
{
  double a, q, r;
  if (!isfinite(x))
    return fmod(x, period);
  a = fabs(x);
  if (a < period)
    return x;
  if (!(a <= period * 0x1p50))
    return fmod(x, period);
  q = trunc(a / period);
  r = fma(-q, period, a);
  if (r < 0.0)
    r += period;
  return copysign(r, x);
}
#endif
