/* Construct the pinned missing-file diagnostic as raw bytes. The native
 * consumer exposes at most AS_MAXCH-1 bytes followed by NUL, even when the
 * complete message is longer. No formatting, locale or numerical work is
 * needed for its five string fragments. Inputs are non-null C strings and
 * do not overlap the caller's writable AS_MAXCH-byte target. */
#include <string.h>

static void swe_missing_file_message(char target[AS_MAXCH], const char *fname,
                                     const char *ephepath)
{
  const char *const pieces[] = {
    "SwissEph file '", fname, "' not found in PATH '", ephepath, "'"
  };
  size_t written = 0;
  for (size_t i = 0; i < sizeof(pieces) / sizeof(pieces[0]); i++) {
    size_t length = strlen(pieces[i]);
    size_t available = AS_MAXCH - 1 - written;
    if (length > available)
      length = available;
    memcpy(target + written, pieces[i], length);
    written += length;
  }
  target[written] = '\0';
  target[AS_MAXCH - 1] = '\0';
}
