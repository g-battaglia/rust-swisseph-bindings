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
#include "missing_file_message.h"

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

static struct observed capture_initialized(native_open function, char *name,
                                           char *path, unsigned char fill,
                                           int omit_diagnostic)
{
  struct observed result = {0};
  memset(result.diagnostic, fill, sizeof(result.diagnostic));
  swed.fidat[0].fnam[0] = '\0';
  FILE *stream = function(0, name, path,
                          omit_diagnostic ? NULL : result.diagnostic);
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
static struct observed compare_observations(struct observed original,
                                            struct observed candidate,
                                            int found, int first_byte,
                                            int read_error)
{
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
  return candidate;
}

static struct observed compare(char *name, char *path, int found, int first_byte,
                               int read_error)
{
  struct observed original = capture_initialized(oracle_swi_fopen, name, path, 0, 0);
  struct observed candidate = capture_initialized(swi_fopen, name, path, 0, 0);
  return compare_observations(original, candidate, found, first_byte, read_error);
}

static void compare_nonzero_diagnostic(char *name, char *path, int found,
                                      int first_byte, int read_error)
{
  struct observed original = capture_initialized(oracle_swi_fopen, name, path, 0xa5, 0);
  struct observed candidate = capture_initialized(swi_fopen, name, path, 0xa5, 0);
  compare_observations(original, candidate, found, first_byte, read_error);
  size_t untouched = 0;
  if (!found) {
    const char *terminator = memchr(candidate.diagnostic, '\0',
                                    sizeof(candidate.diagnostic));
    REQUIRE(terminator != NULL);
    untouched = (size_t)(terminator - candidate.diagnostic) + 1;
  }
  for (size_t index = untouched; index < sizeof(candidate.diagnostic); index++)
    REQUIRE((unsigned char)candidate.diagnostic[index] == 0xa5);
}

static void compare_null_diagnostic(char *name, char *path, int found,
                                     int first_byte, int read_error)
{
  struct observed original = capture_initialized(oracle_swi_fopen, name, path, 0xa5, 1);
  struct observed candidate = capture_initialized(swi_fopen, name, path, 0xa5, 1);
  compare_observations(original, candidate, found, first_byte, read_error);
  for (size_t index = 0; index < sizeof(candidate.diagnostic); index++)
    REQUIRE((unsigned char)candidate.diagnostic[index] == 0xa5);
}

/* serr may alias either input: both implementations finish constructing the
 * message in their local buffer before the final strcpy into caller storage. */
static void compare_aliased_diagnostic(int alias_path)
{
  struct alias_inputs {
    unsigned char before[16];
    char name[AS_MAXCH + 64];
    unsigned char between[16];
    char path[AS_MAXCH + 64];
    unsigned char after[16];
  } original, candidate;
  char original_filename[AS_MAXCH];
  memset(&original, 0xa5, sizeof(original));
  strcpy(original.name, "absent");
  strcpy(original.path, "first:second");
  memcpy(&candidate, &original, sizeof(candidate));
  swed.fidat[0].fnam[0] = '\0';
  REQUIRE(oracle_swi_fopen(0, original.name, original.path,
                         alias_path ? original.path : original.name) == NULL);
  strcpy(original_filename, swed.fidat[0].fnam);
  swed.fidat[0].fnam[0] = '\0';
  REQUIRE(swi_fopen(0, candidate.name, candidate.path,
                   alias_path ? candidate.path : candidate.name) == NULL);
  REQUIRE(strcmp(original_filename, swed.fidat[0].fnam) == 0);
  REQUIRE(memcmp(&original, &candidate, sizeof(original)) == 0);
  const char *diagnostic = alias_path ? candidate.path : candidate.name;
  REQUIRE(strncmp(diagnostic, "SwissEph file '",
                  sizeof("SwissEph file '") - 1) == 0);
  for (size_t index = 0; index < sizeof(candidate.before); index++) {
    REQUIRE(candidate.before[index] == 0xa5);
    REQUIRE(candidate.between[index] == 0xa5);
    REQUIRE(candidate.after[index] == 0xa5);
    REQUIRE(original.before[index] == 0xa5);
    REQUIRE(original.between[index] == 0xa5);
    REQUIRE(original.after[index] == 0xa5);
  }
  for (size_t index = AS_MAXCH; index < sizeof(candidate.name); index++) {
    REQUIRE((unsigned char)candidate.name[index] == 0xa5);
    REQUIRE((unsigned char)candidate.path[index] == 0xa5);
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

/* The unmodified consumer's temporary buffer is only 512 bytes. Arbitrarily
 * long pieces belong to this standalone helper control, whose independent
 * snprintf oracle has space for both complete inputs before truncation. */
enum { MESSAGE_INPUT_CAPACITY = 4096, MESSAGE_GUARD_BYTES = 16 };
static unsigned int message_cases, message_negative_controls;

static int same_message(const unsigned char *actual, const char *expected,
                        size_t length)
{
  return memcmp(actual, expected, length + 1) == 0;
}

static void compare_message(const char *name, const char *path)
{
  struct {
    unsigned char before[MESSAGE_GUARD_BYTES];
    unsigned char target[AS_MAXCH];
    unsigned char after[MESSAGE_GUARD_BYTES];
  } guarded;
  char expected[2 * MESSAGE_INPUT_CAPACITY + 128];
  int full_length = snprintf(expected, sizeof(expected),
                            "SwissEph file '%s' not found in PATH '%s'",
                            name, path);
  REQUIRE(full_length >= 0 && (size_t)full_length < sizeof(expected));
  size_t length = (size_t)full_length;
  if (length >= AS_MAXCH)
    length = AS_MAXCH - 1;
  expected[length] = '\0';

  memset(&guarded, 0xa5, sizeof(guarded));
  swe_missing_file_message((char *)guarded.target, name, path);
  REQUIRE(same_message(guarded.target, expected, length));
  REQUIRE(guarded.target[length] == '\0');
  REQUIRE(memchr(guarded.target, '\0', length) == NULL);
  for (size_t index = length + 1; index < AS_MAXCH - 1; index++)
    REQUIRE(guarded.target[index] == 0xa5);
  REQUIRE(guarded.target[AS_MAXCH - 1] == '\0');
  for (size_t index = 0; index < MESSAGE_GUARD_BYTES; index++) {
    REQUIRE(guarded.before[index] == 0xa5);
    REQUIRE(guarded.after[index] == 0xa5);
  }

  /* Mutate the independent expectation only in RAM. Every comparison must
   * reject a changed byte rather than passing through a vacuous predicate. */
  expected[0] ^= 1;
  REQUIRE(!same_message(guarded.target, expected, length));
  message_negative_controls++;
  message_cases++;
}

static void message_piece_boundaries(void)
{
  char name[MESSAGE_INPUT_CAPACITY + 1];
  char path[MESSAGE_INPUT_CAPACITY + 1];
  memset(name, 'n', sizeof(name));
  memset(path, 'p', sizeof(path));

  /* Sweep every split through the fixed prefix, filename, separator, path
   * and suffix around the 255-byte output limit, including empty pieces. */
  for (size_t name_length = 0; name_length <= AS_MAXCH + 1; name_length++) {
    name[name_length] = '\0';
    for (size_t path_length = 0; path_length <= AS_MAXCH + 1; path_length++) {
      path[path_length] = '\0';
      compare_message(name, path);
      path[path_length] = 'p';
    }
    name[name_length] = 'n';
  }

  name[MESSAGE_INPUT_CAPACITY] = '\0';
  path[MESSAGE_INPUT_CAPACITY] = '\0';
  compare_message(name, path);
  compare_message(name, "");
  compare_message("", path);
  compare_message("'", "'");
  compare_message("%s%n", "%p%%");

  /* Treat filenames as bytes: truncation may split a UTF-8 sequence and must
   * also preserve high bytes when plain char is signed on the target ABI. */
  static const unsigned char utf8[] = {0xc3, 0xa9, 0xe2, 0x82, 0xac,
                                      0xf0, 0x9f, 0x8c, 0x8c};
  static const unsigned char high_bytes[] = {0x80, 0xff, 0xfe, 0x81};
  for (size_t index = 0; index < MESSAGE_INPUT_CAPACITY; index++) {
    name[index] = (char)utf8[index % sizeof(utf8)];
    path[index] = (char)high_bytes[index % sizeof(high_bytes)];
  }
  for (size_t length = 0; length <= AS_MAXCH + 1; length++) {
    char saved_name = name[length], saved_path = path[length];
    name[length] = '\0';
    path[length] = '\0';
    compare_message(name, path);
    compare_message(path, name);
    name[length] = saved_name;
    path[length] = saved_path;
  }
  compare_message(name, path);
}

int main(int argc, char **argv)
{
  REQUIRE(geteuid() != 0); /* Permission controls require the ordinary user. */
  REQUIRE(argc == 2);
  message_piece_boundaries();
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

  /* Exercise the rewritten production diagnostic and its original C oracle
   * below the native temporary-buffer limit, including truncated messages. */
  char missing_path[AS_MAXCH - 2];
  memset(missing_path, 'x', sizeof(missing_path) - 1);
  missing_path[sizeof(missing_path) - 1] = '\0';
  struct observed missing = compare("q", missing_path, 0, 0, 0);
  REQUIRE(strncmp(missing.diagnostic, "SwissEph file '",
                  sizeof("SwissEph file '") - 1) == 0);
  REQUIRE(strlen(missing.diagnostic) == AS_MAXCH - 1);
  compare_nonzero_diagnostic("q", missing_path, 0, 0, 0);
  compare_null_diagnostic("q", missing_path, 0, 0, 0);
  compare("absent-\xc3\xa9", "missing-\xe2\x82\xac", 0, 0, 0);
  compare_nonzero_diagnostic("absent", path, 0, 0, 0);
  compare_nonzero_diagnostic("input", path, 1, 'B', 0);
  compare_null_diagnostic("absent", path, 0, 0, 0);
  compare_null_diagnostic("input", path, 1, 'B', 0);
  compare_aliased_diagnostic(0);
  compare_aliased_diagnostic(1);
  char twenty_paths[AS_MAXCH];
  size_t used = 0;
  for (int index = 0; index < 20; index++) {
    int written = snprintf(twenty_paths + used, sizeof(twenty_paths) - used,
                           "%smissing%02d", index ? ":" : "", index);
    REQUIRE(written > 0 && (size_t)written < sizeof(twenty_paths) - used);
    used += (size_t)written;
  }
  compare("input", twenty_paths, 0, 0, 0);
  char *last_path = strrchr(twenty_paths, ':');
  REQUIRE(last_path != NULL);
  strcpy(last_path + 1, "first");
  compare("input", twenty_paths, 1, 'B', 0);
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
  printf("PASS read-only file control: %u filesystem cases, %u bounded-message cases, %u negative controls, complete diagnostics, descriptor ownership and non-POSIX delegation\n",
         cases, message_cases, message_negative_controls);
  return 0;
}
