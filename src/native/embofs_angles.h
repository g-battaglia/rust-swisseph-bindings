/* Evaluate two independent native mean-angle cubics. Time is Julian centuries
 * from J1900; each result is an unreduced angle in degrees. Coefficient pairs
 * follow c3, c2, c1, c0 in c3*T^3 + c2*T^2 + c1*T + c0. Three complete stages
 * retain the native FMA graph; actual contraction and all modes need controls. */
#ifndef SWE_EMBOFS_ANGLES_H
#define SWE_EMBOFS_ANGLES_H

typedef struct {
  double first, second;
} swe_embofs_pair;

/* Compiler vectors name independent doubles, not a CPU instruction set. The
 * function ABI and fallback stay scalar; no pointer aliases or alignment
 * requirements are introduced on the caller's position or native state. */
#if !defined(SWE_EMBOFS_FORCE_SCALAR)
# if defined(__has_attribute)
#  if __has_attribute(vector_size)
#   define SWE_EMBOFS_USE_VECTOR 1
#  endif
# elif defined(__GNUC__) && __GNUC__ >= 4
#  define SWE_EMBOFS_USE_VECTOR 1
# endif
#endif
#ifndef SWE_EMBOFS_USE_VECTOR
# define SWE_EMBOFS_USE_VECTOR 0
#endif

#if SWE_EMBOFS_USE_VECTOR
typedef double swe_embofs_lanes __attribute__((vector_size(16)));
#endif

static inline swe_embofs_pair swe_embofs_cubic_pair(
    double time, swe_embofs_pair cubic, swe_embofs_pair quadratic,
    swe_embofs_pair linear, swe_embofs_pair constant)
{
#if SWE_EMBOFS_USE_VECTOR
  const swe_embofs_lanes T = {time, time};
  const swe_embofs_lanes c3 = {cubic.first, cubic.second};
  const swe_embofs_lanes c2 = {quadratic.first, quadratic.second};
  const swe_embofs_lanes c1 = {linear.first, linear.second};
  const swe_embofs_lanes c0 = {constant.first, constant.second};
  const swe_embofs_lanes stage1 = c3 * T + c2;
# ifdef SWE_EMBOFS_TRACE_STAGE
  SWE_EMBOFS_TRACE_STAGE(1, stage1[0], stage1[1]);
# endif
  const swe_embofs_lanes stage2 = stage1 * T + c1;
# ifdef SWE_EMBOFS_TRACE_STAGE
  SWE_EMBOFS_TRACE_STAGE(2, stage2[0], stage2[1]);
# endif
  const swe_embofs_lanes stage3 = stage2 * T + c0;
# ifdef SWE_EMBOFS_TRACE_STAGE
  SWE_EMBOFS_TRACE_STAGE(3, stage3[0], stage3[1]);
# endif
  const swe_embofs_pair result = {stage3[0], stage3[1]};
  return result;
#else
  const swe_embofs_pair stage1 = {
    cubic.first * time + quadratic.first,
    cubic.second * time + quadratic.second
  };
# ifdef SWE_EMBOFS_TRACE_STAGE
  SWE_EMBOFS_TRACE_STAGE(1, stage1.first, stage1.second);
# endif
  const swe_embofs_pair stage2 = {
    stage1.first * time + linear.first,
    stage1.second * time + linear.second
  };
# ifdef SWE_EMBOFS_TRACE_STAGE
  SWE_EMBOFS_TRACE_STAGE(2, stage2.first, stage2.second);
# endif
  const swe_embofs_pair stage3 = {
    stage2.first * time + constant.first,
    stage2.second * time + constant.second
  };
# ifdef SWE_EMBOFS_TRACE_STAGE
  SWE_EMBOFS_TRACE_STAGE(3, stage3.first, stage3.second);
# endif
  return stage3;
#endif
}

#endif /* SWE_EMBOFS_ANGLES_H */
