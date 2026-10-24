/* Open a read-only native data file. The caller's filename, mode and search
 * order stay unchanged. POSIX can reject an absent path before allocating a
 * FILE; fdopen owns a successful descriptor, and only failure closes it here.
 * Other platforms retain fopen. Errno, cancellation and resource-error
 * precedence are not part of this helper's returned-stream contract. */
#if defined(__unix__) || defined(__APPLE__)
#include <fcntl.h>
#include <unistd.h>
#endif

static FILE *swe_open_readonly_file(const char *path)
{
#if defined(__unix__) || defined(__APPLE__)
  int descriptor = open(path, O_RDONLY);
  if (descriptor < 0)
    return NULL;
  FILE *stream = fdopen(descriptor, BFILE_R_ACCESS);
  if (stream == NULL)
    close(descriptor);
  return stream;
#else
  return fopen(path, BFILE_R_ACCESS);
#endif
}
