/* SPDX-License-Identifier: AGPL-3.0-only
 * Lossless ASCII-space removal for the already truncated v1 catalog name.
 * No catalog, match, configuration or astronomical value is retained here.
 */
#ifndef SWE_FIXED_STAR_ASCII_SPACE_COMPACTION_H
#define SWE_FIXED_STAR_ASCII_SPACE_COMPACTION_H
static void swe_compact_fixed_star_ascii_spaces(char *name) {
  char *read = name;
  char *write = name;
  while (*read != '\0') {
    if (*read != ' ')
      *write++ = *read;
    read++;
  }
  *write = '\0';
}
#endif
