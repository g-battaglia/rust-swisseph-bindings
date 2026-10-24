/* Independent coordinates retain their native product, fused contribution and
 * ordered total-add stages. Only private arithmetic uses compiler vector lanes;
 * coefficients, periods, trig calls, coordinate order and public ABI stay scalar. */
typedef struct {
  double first, second;
} swe_precession_pair;

typedef struct {
  double period;
  swe_precession_pair cosine, sine;
} swe_precession_term;

/* Generic compiler vectors name two independent doubles, not a CPU instruction
 * set. Other compilers use the scalar body. Independent controls also define
 * SWE_PRECESSION_FORCE_SCALAR to exercise that exact fallback on vector hosts. */
#if !defined(SWE_PRECESSION_FORCE_SCALAR)
# if defined(__has_attribute)
#  if __has_attribute(vector_size)
#   define SWE_PRECESSION_USE_VECTOR 1
#  endif
# elif defined(__GNUC__) && __GNUC__ >= 4
#  define SWE_PRECESSION_USE_VECTOR 1
# endif
#endif
#ifndef SWE_PRECESSION_USE_VECTOR
# define SWE_PRECESSION_USE_VECTOR 0
#endif

#if SWE_PRECESSION_USE_VECTOR
typedef double swe_precession_lanes __attribute__((vector_size(16)));
#endif

static inline swe_precession_pair swe_precession_accumulate(
    swe_precession_pair total, double cosine, double sine,
    const swe_precession_term *term)
{
#if SWE_PRECESSION_USE_VECTOR
  /* Scalar field reads construct the lanes without type-punning or a stronger
   * alignment requirement on the original coefficient records. The vector type
   * never appears in a function parameter, return type, or stored table. */
  const swe_precession_lanes sine_coefficients = {
    term->sine.first, term->sine.second
  };
  const swe_precession_lanes cosine_coefficients = {
    term->cosine.first, term->cosine.second
  };
  const swe_precession_lanes sine_lanes = {sine, sine};
  const swe_precession_lanes cosine_lanes = {cosine, cosine};
  const swe_precession_lanes incoming = {total.first, total.second};
  /* Keep three full expressions: rounded sine product, the native contracted
   * cosine sum, then the separate addition to each incoming coordinate total.
   * The compiler's actual contraction and rounding still require qualification. */
  const swe_precession_lanes sine_product = sine_lanes * sine_coefficients;
  const swe_precession_lanes contribution =
      cosine_lanes * cosine_coefficients + sine_product;
  const swe_precession_lanes updated = incoming + contribution;
  const swe_precession_pair result = {updated[0], updated[1]};
  return result;
#else
  const swe_precession_pair sine_product = {
    sine * term->sine.first,
    sine * term->sine.second
  };
  const swe_precession_pair contribution = {
    cosine * term->cosine.first + sine_product.first,
    cosine * term->cosine.second + sine_product.second
  };
  const swe_precession_pair updated = {
    total.first + contribution.first,
    total.second + contribution.second
  };
  return updated;
#endif
}
