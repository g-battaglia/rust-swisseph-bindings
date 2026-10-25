/* Two independent angular channels in native coefficient units. Dimensionless
 * T is supplied by the original dispatcher; radius and angular recurrence never
 * enter this helper. Values remain private until the existing scalar-pair output.
 * Whole-object memcpy loads preserve the original coefficient record, alignment
 * and strict aliasing: no cast from a first member to a vector is permitted. */
#ifndef SWE_MOSHIER_COORDINATE_PAIR_H
#define SWE_MOSHIER_COORDINATE_PAIR_H
#include <stddef.h>
#include <string.h>

_Static_assert(sizeof(struct swe_moshier_pair) == 2 * sizeof(double),
               "coordinate record must contain exactly two native doubles");
_Static_assert(offsetof(struct swe_moshier_pair, longitude) == 0 &&
               offsetof(struct swe_moshier_pair, latitude) == sizeof(double),
               "coordinate record field order must remain native");

/* AArch64 provides the required double-precision NEON arithmetic. Little-endian
 * object layout maps the first stored double to lane 0. Other architectures or
 * compilers without a known byte order retain the same scalar expressions.
 * There is no CPU probe, target flag, external dependency or control-test macro. */
#if (defined(__aarch64__) || defined(__arm64__)) &&     defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
#include <arm_neon.h>
#define SWE_MOSHIER_COORDINATE_NEON 1
typedef float64x2_t swe_moshier_coordinate;
#else
#define SWE_MOSHIER_COORDINATE_NEON 0
typedef struct swe_moshier_pair swe_moshier_coordinate;
#endif
_Static_assert(sizeof(swe_moshier_coordinate) == sizeof(struct swe_moshier_pair),
               "private arithmetic value must retain both complete components");

static inline swe_moshier_coordinate swe_moshier_coordinate_load(
    const struct swe_moshier_pair *source)
{
  swe_moshier_coordinate value;
  memcpy(&value, source, sizeof(value));
  return value;
}

static inline void swe_moshier_coordinate_store(struct swe_moshier_pair *out,
                                                swe_moshier_coordinate value)
{
  memcpy(out, &value, sizeof(value));
}

static inline swe_moshier_coordinate swe_moshier_coordinate_zero(void)
{
#if SWE_MOSHIER_COORDINATE_NEON
  return vdupq_n_f64(0.0);
#else
  const swe_moshier_coordinate value = {0.0, 0.0};
  return value;
#endif
}

/* One rounded FMA per lane, preserving each native Horner stage. Coefficients
 * enter as their unchanged complete record, never as separately packed fields. */
static inline swe_moshier_coordinate swe_moshier_coordinate_horner(
    swe_moshier_coordinate value, double time,
    const struct swe_moshier_pair *coefficient)
{
  const swe_moshier_coordinate c = swe_moshier_coordinate_load(coefficient);
#if SWE_MOSHIER_COORDINATE_NEON
  return vfmaq_n_f64(c, value, time);
#else
  const swe_moshier_coordinate updated = {
    value.longitude * time + c.longitude,
    value.latitude * time + c.latitude
  };
  return updated;
#endif
}

/* The rounded sine product, cosine FMA and incoming-total ADD stay separate
 * calls/full expressions in the evaluator. No horizontal reduction occurs. */
static inline swe_moshier_coordinate swe_moshier_coordinate_product(
    swe_moshier_coordinate sine, double sv)
{
#if SWE_MOSHIER_COORDINATE_NEON
  return vmulq_n_f64(sine, sv);
#else
  const swe_moshier_coordinate product = {sine.longitude * sv, sine.latitude * sv};
  return product;
#endif
}

static inline swe_moshier_coordinate swe_moshier_coordinate_contribution(
    swe_moshier_coordinate cosine, double cv, swe_moshier_coordinate product)
{
#if SWE_MOSHIER_COORDINATE_NEON
  return vfmaq_n_f64(product, cosine, cv);
#else
  const swe_moshier_coordinate contribution = {
    cosine.longitude * cv + product.longitude,
    cosine.latitude * cv + product.latitude
  };
  return contribution;
#endif
}

static inline swe_moshier_coordinate swe_moshier_coordinate_add(
    swe_moshier_coordinate incoming, swe_moshier_coordinate contribution)
{
#if SWE_MOSHIER_COORDINATE_NEON
  return vaddq_f64(incoming, contribution);
#else
  const swe_moshier_coordinate total = {
    incoming.longitude + contribution.longitude,
    incoming.latitude + contribution.latitude
  };
  return total;
#endif
}

/* Only the polynomial longitude crosses the original scalar mods3600 site.
 * Reading/replacing that component performs no additional FP arithmetic; the
 * latitude lane is preserved. Periodic terms never extract individual lanes. */
static inline double swe_moshier_coordinate_longitude(swe_moshier_coordinate value)
{
#if SWE_MOSHIER_COORDINATE_NEON
  return vgetq_lane_f64(value, 0);
#else
  return value.longitude;
#endif
}

static inline swe_moshier_coordinate swe_moshier_coordinate_set_longitude(
    swe_moshier_coordinate value, double longitude)
{
#if SWE_MOSHIER_COORDINATE_NEON
  return vsetq_lane_f64(longitude, value, 0);
#else
  value.longitude = longitude;
  return value;
#endif
}
#endif
