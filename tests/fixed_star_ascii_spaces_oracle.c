/* SPDX-License-Identifier: AGPL-3.0-only
 * Exhaustive nonastronomical equivalence control for the owned compactor.
 */
#include <stdio.h>
#include <string.h>
#include "fixed_star_ascii_space_compaction.h"

static unsigned long cases;

static int compare(const char *input) {
  char previous[256], compacted[256], *space;
  strcpy(previous, input);
  strcpy(compacted, input);
  /* Original repeated deletion, with the defined overlapping-copy operation. */
  while ((space = strchr(previous, ' ')) != NULL)
    memmove(space, space + 1, strlen(space + 1) + 1);
  swe_compact_fixed_star_ascii_spaces(compacted);
  cases++;
  return strcmp(previous, compacted) == 0;
}

static int enumerate(char *input, int position, int length) {
  static const unsigned char alphabet[] = {' ', 'A', 'b', '\t', ',', 0x80, 0xff};
  unsigned int index;
  if (position == length) {
    input[position] = '\0';
    return compare(input);
  }
  for (index = 0; index < sizeof(alphabet); index++) {
    input[position] = (char) alphabet[index];
    if (!enumerate(input, position + 1, length))
      return 0;
  }
  return 1;
}

int main(void) {
  char input[256];
  int length, index;
  for (length = 0; length <= 6; length++)
    if (!enumerate(input, 0, length))
      return 1;
  for (length = 1; length < 256; length++) {
    for (index = 0; index < length; index++)
      input[index] = (index % 3 == 0) ? ' ' : (index % 3 == 1) ? 'X' : '\t';
    input[length] = '\0';
    if (!compare(input))
      return 2;
    memset(input, ' ', (size_t) length);
    if (!compare(input))
      return 3;
  }
  printf("%lu\n", cases);
  return 0;
}
