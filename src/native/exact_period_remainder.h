/* SPDX-License-Identifier: AGPL-3.0-only */
#ifndef SWE_EXACT_PERIOD_REMAINDER_H
#define SWE_EXACT_PERIOD_REMAINDER_H

#include <float.h>
#include <math.h>
#include <stdint.h>
#include <string.h>

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
#if defined(__APPLE__) && defined(__aarch64__) && defined(__clang__) && \
    !defined(SWE_EXACT_PERIOD_FORCE_SCALAR_ADMISSION)
  /* The Apple arm64 ABI represents double as IEEE binary64, with the same
   * little-endian byte order as uint64_t. Positive encodings are ordered.
   * The two admitted periods are positive normals; adding 50 to their
   * exponent gives the exact period*2^50 bound, without floating arithmetic.
   * NaN/infinity encodings also exceed that finite bound and retain libc.
   * Other representations keep the original admission below. */
  uint64_t magnitude, upper;
  _Static_assert(sizeof(double) == sizeof(uint64_t) && FLT_RADIX == 2 &&
                 DBL_MANT_DIG == 53 && DBL_MAX_EXP == 1024,
                 "Apple arm64 admission requires IEEE binary64");
  memcpy(&magnitude, &x, sizeof magnitude);
  memcpy(&upper, &period, sizeof upper);
  magnitude &= UINT64_MAX >> 1;
  upper += UINT64_C(50) << 52;
  if (magnitude > upper)
    return fmod(x, period);
  a = fabs(x);
  if (a < period)
    return x;
#else
  if (!isfinite(x))
    return fmod(x, period);
  a = fabs(x);
  if (a < period)
    return x;
  if (!(a <= period * 0x1p50))
    return fmod(x, period);
#endif
  q = trunc(a * reciprocal);
  r = fma(-q, period, a);
  if (r < 0.0)
    r += period;
  return copysign(r, x);
}
#endif
