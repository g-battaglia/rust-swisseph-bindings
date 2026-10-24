/* Filesystem inputs and scalar checks only; no astronomical output is stored. */
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include "swephexp.h"
#include "sweph.h"
#include "readonly_file_open.h"

#define REQUIRE(condition) do { \
  if (!(condition)) { fprintf(stderr, "file control failed at line %d\n", __LINE__); exit(1); } \
} while (0)

extern FILE *oracle_swi_fopen(int, char *, char *, char *);
typedef FILE *(*native_open)(int, char *, char *, char *);

/* Force the FILE allocation failure after a real successful open. Only this
 * test copy redirects fdopen/close; the production translation unit does not. */
static int owned_descriptor, failure_calls, owned_closes;
static FILE *fail_fdopen(int descriptor, const char *mode)
{
  REQUIRE(fcntl(descriptor, F_GETFD) >= 0);
  REQUIRE(strcmp(mode, BFILE_R_ACCESS) == 0);
  owned_descriptor = descriptor;
  failure_calls++;
  errno = ENOMEM;
  return NULL;
}
static int close_owned(int descriptor)
{
  REQUIRE(descriptor == owned_descriptor);
  owned_closes++;
  return close(descriptor);
}
#define swe_open_readonly_file allocation_failure_open
#define fdopen fail_fdopen
#define close close_owned
#include "readonly_file_open.h"
#undef close
#undef fdopen
#undef swe_open_readonly_file

/* System headers are already loaded. Select only this helper's non-POSIX
 * branch and verify that it delegates unchanged mode/path to ordinary fopen. */
static int fallback_calls;
static FILE *fallback_fopen(const char *path, const char *mode)
{
  REQUIRE(strcmp(mode, BFILE_R_ACCESS) == 0);
  fallback_calls++;
  return fopen(path, mode);
}
#undef __unix__
#undef __APPLE__
#define swe_open_readonly_file nonposix_open
#define fopen fallback_fopen
#include "readonly_file_open.h"
#undef fopen
#undef swe_open_readonly_file

struct observed {
  int found, read_error, seek_status, close_status, descriptor_flags, access_mode;
  long offset;
  size_t count;
  unsigned char bytes[8];
  char filename[AS_MAXCH], diagnostic[256];
};

static struct observed capture(native_open function, char *name, char *path)
{
  struct observed result = {0};
  swed.fidat[0].fnam[0] = '\0';
  FILE *stream = function(0, name, path, result.diagnostic);
  strcpy(result.filename, swed.fidat[0].fnam);
  result.found = stream != NULL;
  if (stream == NULL) return result;
  result.descriptor_flags = fcntl(fileno(stream), F_GETFD);
  result.access_mode = fcntl(fileno(stream), F_GETFL) & O_ACCMODE;
  result.count = fread(result.bytes, 1, sizeof(result.bytes), stream);
  result.read_error = !!ferror(stream);
  result.seek_status = fseek(stream, 0, SEEK_SET);
  result.offset = ftell(stream);
  result.close_status = fclose(stream);
  return result;
}

static unsigned int cases;
static void compare(char *name, char *path, int found, int first_byte, int read_error)
{
  struct observed original = capture(oracle_swi_fopen, name, path);
  struct observed candidate = capture(swi_fopen, name, path);
  REQUIRE(original.found == found && candidate.found == found);
  REQUIRE(original.count == candidate.count);
  REQUIRE(original.read_error == candidate.read_error);
  REQUIRE(original.seek_status == candidate.seek_status);
  REQUIRE(original.offset == candidate.offset);
  REQUIRE(original.close_status == candidate.close_status);
  REQUIRE(original.descriptor_flags == candidate.descriptor_flags);
  REQUIRE(original.access_mode == candidate.access_mode);
  REQUIRE(memcmp(original.bytes, candidate.bytes, sizeof(original.bytes)) == 0);
  REQUIRE(memcmp(original.diagnostic, candidate.diagnostic, 256) == 0);
  REQUIRE(strcmp(original.filename, candidate.filename) == 0);
  if (found) {
    REQUIRE(candidate.access_mode == O_RDONLY);
    REQUIRE(candidate.read_error == read_error);
    if (!read_error) REQUIRE(candidate.count && candidate.bytes[0] == first_byte);
  }
  cases++;
}

static void write_file(const char *path, const char *data)
{
  FILE *stream = fopen(path, "wb");
  REQUIRE(stream != NULL);
  REQUIRE(fwrite(data, 1, strlen(data), stream) == strlen(data));
  REQUIRE(fclose(stream) == 0);
}

int main(int argc, char **argv)
{
  REQUIRE(geteuid() != 0); /* Permission controls require the ordinary user. */
  REQUIRE(argc == 2);
  char directory[PATH_MAX];
  int length = snprintf(directory, sizeof(directory), "%s/files-XXXXXX", argv[1]);
  REQUIRE(length > 0 && (size_t)length < sizeof(directory));
  REQUIRE(mkdtemp(directory) != NULL);
  REQUIRE(chdir(directory) == 0);
  REQUIRE(mkdir("first", 0700) == 0 && mkdir("second", 0700) == 0);
  char path[] = "first:second";
  compare("input", path, 0, 0, 0);
  write_file("first/input", "A-first");
  compare("input", path, 1, 'A', 0);
  REQUIRE(unlink("first/input") == 0);
  compare("input", path, 0, 0, 0);
  write_file("second/input", "B-second");
  compare("input", path, 1, 'B', 0);
  write_file("first/input", "A-first");
  compare("input", path, 1, 'A', 0);
  char reversed[] = "second:first";
  compare("input", reversed, 1, 'B', 0);
  REQUIRE(rename("second/input", "first/input") == 0);
  compare("input", path, 1, 'B', 0);
  REQUIRE(symlink("missing", "first/link") == 0);
  compare("link", path, 0, 0, 0);
  REQUIRE(unlink("first/link") == 0 && symlink("input", "first/link") == 0);
  compare("link", path, 1, 'B', 0);
  compare("first", ".", 1, 0, 1); /* Read-only directory stream: read error. */
  compare("first/input", ".", 1, 'B', 0);
  REQUIRE(chmod("first/input", 0000) == 0);
  compare("input", path, 0, 0, 0);
  REQUIRE(chmod("first/input", 0600) == 0);
  compare("input", path, 1, 'B', 0);
  char long_path[AS_MAXCH];
  memset(long_path, 'x', sizeof(long_path) - 1);
  long_path[sizeof(long_path) - 1] = '\0';
  compare("input", long_path, 0, 0, 0);
  char long_name[AS_MAXCH + 1];
  memset(long_name, 'x', sizeof(long_name) - 1);
  long_name[sizeof(long_name) - 1] = '\0';
  compare(long_name, path, 0, 0, 0);

  FILE *stream = swe_open_readonly_file("first/input");
  REQUIRE(stream != NULL && fgetc(stream) == 'B');
  REQUIRE(fseek(stream, 2, SEEK_SET) == 0 && ftell(stream) == 2);
  REQUIRE(fclose(stream) == 0);
  REQUIRE(allocation_failure_open("first/input") == NULL);
  REQUIRE(failure_calls == 1 && owned_closes == 1);
  errno = 0;
  REQUIRE(fcntl(owned_descriptor, F_GETFD) == -1 && errno == EBADF);
  REQUIRE(allocation_failure_open("absent") == NULL);
  REQUIRE(failure_calls == 1 && owned_closes == 1);
  stream = nonposix_open("first/input");
  REQUIRE(stream != NULL && fgetc(stream) == 'B');
  REQUIRE(fclose(stream) == 0 && fallback_calls == 1);
  REQUIRE(nonposix_open("absent") == NULL && fallback_calls == 2);

  REQUIRE(unlink("first/link") == 0 && unlink("first/input") == 0);
  REQUIRE(rmdir("first") == 0 && rmdir("second") == 0);
  REQUIRE(chdir("/") == 0 && rmdir(directory) == 0);
  printf("PASS read-only file control: %u filesystem cases, complete diagnostics, descriptor ownership and non-POSIX delegation\n", cases);
  return 0;
}
