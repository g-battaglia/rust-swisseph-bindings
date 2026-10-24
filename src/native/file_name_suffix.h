/* Append the native nonnegative century and fixed file suffix. Decimal width
 * is at least two, never a truncation: 0 becomes 00 and values above 99 keep
 * every digit. The caller already owns enough writable bytes, as for the
 * original sprintf, and supplies the pointer after its unchanged prefix.
 * Only bytes through the final NUL are written; no locale or astronomy work
 * belongs to this fixed presentation step. */
#include <limits.h>
#include <string.h>

static void swe_append_file_century(char *output, unsigned int century)
{
  if (century < 100u) {
    /* Minimum width two fixes both positions, including the leading zero. */
    *output++ = (char)('0' + century / 10u);
    *output++ = (char)('0' + century % 10u);
  } else {
    /* ceil(bit width / 3) conservatively bounds the decimal digit count.
     * Larger values keep every digit; the local array is constant-sized. */
    char digits[(sizeof(unsigned int) * CHAR_BIT + 2) / 3];
    size_t used = 0;
    do {
      digits[used++] = (char)('0' + century % 10u);
      century /= 10u;
    } while (century != 0);
    while (used != 0)
      *output++ = digits[--used];
  }
  *output++ = '.';
  memcpy(output, SE_FILE_SUFFIX, sizeof(SE_FILE_SUFFIX));
}
