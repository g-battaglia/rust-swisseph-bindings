/* Private preparation for the two nonoverlapping vectors of ONE operation.
 * Initialize at the adjacent position/speed call site, pass identical Julian
 * day, flags and direction, and discard on return. No state survives the call.
 * The original selector and scalar fallback remain in swephlib.c. */
#ifndef SWE_PRECESSION_FRAME_H
#define SWE_PRECESSION_FRAME_H
struct swe_precession_frame {
  double matrix[9];
  int prepared;
};
void swe_precess_with_frame(double *vector, double day, int32 flags,
                           int direction, struct swe_precession_frame *frame);
#endif
