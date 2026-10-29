/* SWISSEPH

   SWISSEPH modules that may be useful for other applications
   e.g. chopt.c, venus.c, swetest.c

  Authors: Dieter Koch and Alois Treindl, Astrodienst Zurich

   coordinate transformations
   obliquity of ecliptic
   nutation
   precession
   delta t
   sidereal time
   setting or getting of tidal acceleration of moon
   chebyshew interpolation
   ephemeris file name generation
   cyclic redundancy checksum CRC
   modulo and normalization functions

**************************************************************/
/* Copyright (C) 1997 - 2021 Astrodienst AG, Switzerland.  All rights reserved.

  License conditions
  ------------------

  This file is part of Swiss Ephemeris.

  Swiss Ephemeris is distributed with NO WARRANTY OF ANY KIND.  No author
  or distributor accepts any responsibility for the consequences of using it,
  or for whether it serves any particular purpose or works at all, unless he
  or she says so in writing.  

  Swiss Ephemeris is made available by its authors under a dual licensing
  system. The software developer, who uses any part of Swiss Ephemeris
  in his or her software, must choose between one of the two license models,
  which are
  a) GNU Affero General Public License (AGPL)
  b) Swiss Ephemeris Professional License

  The choice must be made before the software developer distributes software
  containing parts of Swiss Ephemeris to others, and before any public
  service using the developed software is activated.

  If the developer choses the AGPL software license, he or she must fulfill
  the conditions of that license, which includes the obligation to place his
  or her whole software project under the AGPL or a compatible license.
  See https://www.gnu.org/licenses/agpl-3.0.html

  If the developer choses the Swiss Ephemeris Professional license,
  he must follow the instructions as found in http://www.astro.com/swisseph/ 
  and purchase the Swiss Ephemeris Professional Edition from Astrodienst
  and sign the corresponding license contract.

  The License grants you the right to use, copy, modify and redistribute
  Swiss Ephemeris, but only under certain conditions described in the License.
  Among other things, the License requires that the copyright notices and
  this notice be preserved on all copies.

  Authors of the Swiss Ephemeris: Dieter Koch and Alois Treindl

  The authors of Swiss Ephemeris have no control or influence over any of
  the derived works, i.e. over software or services created by other
  programmers which use Swiss Ephemeris functions.

  The names of the authors or of the copyright holder (Astrodienst) must not
  be used for promoting any software, product or service which uses or contains
  the Swiss Ephemeris. This copyright notice is the ONLY place where the
  names of the authors can legally appear, except in cases where they have
  given special permission in writing.

  The trademarks 'Swiss Ephemeris' and 'Swiss Ephemeris inside' may be used
  for promoting such software, products or services.
*/

/* Versioned output contract from Swiss Ephemeris 2.10.03.
 * Original: libswe/swephlib.c in the pyswisseph 2.10.3.2 source distribution.
 * Original file SHA256: d41eb2f396163fedd574749de02a25e8547b4e1e4c66783d47ccc0c7e4885a05
 * Only function names are changed. The pinned engine remains unmodified.
 */

#include "swephexp.h"
#include "sweph.h"
#include "swephlib.h"

static void split_deg_nakshatra_2_10_03(double ddeg, int32 roundflag, int32 *ideg, int32 *imin, int32 *isec, double *dsecfr, int32 *inak)
{
  double dadd = 0;
  double dnakshsize = 13.33333333333333;
  double ddeghelp = fmod(ddeg, dnakshsize);
  *inak = 1;
  if (ddeg < 0) {
    *inak = -1;
    ddeg = 0;
  }
  // Sheoran "Vedic" ayanamsha: 0 Aries = 3°20 Ashvini
  if ((swed.sidd.sid_mode & SE_SIDM_TRUE_SHEORAN) == SE_SIDM_TRUE_SHEORAN)
    ddeg = swe_degnorm(ddeg + 3.33333333333333);
  if (roundflag & SE_SPLIT_DEG_ROUND_DEG) {
    dadd = 0.5;
  } else if (roundflag & SE_SPLIT_DEG_ROUND_MIN) {
    dadd = 0.5 / 60;
  } else if (roundflag & SE_SPLIT_DEG_ROUND_SEC) {
    dadd = 0.5 / 3600;
  }
  if (roundflag & SE_SPLIT_DEG_KEEP_DEG) {
    if ((int32) (ddeghelp + dadd) - (int32) ddeghelp > 0)
      dadd = 0;
  } else if (roundflag & SE_SPLIT_DEG_KEEP_SIGN) {
    if (ddeghelp + dadd >= dnakshsize)
      dadd = 0;
  }
  ddeg += dadd;
  *inak = (int32) (ddeg / dnakshsize);
  if (*inak == 27) *inak = 0; // with rounding up from 359.9999
  ddeg = fmod(ddeg, dnakshsize);
  *ideg = (int32) ddeg;
  ddeg -= *ideg;
  *imin = (int32) (ddeg * 60);
  ddeg -= *imin / 60.0;
  *isec = (int32) (ddeg * 3600);
  if (!(roundflag & (SE_SPLIT_DEG_ROUND_DEG | SE_SPLIT_DEG_ROUND_MIN | SE_SPLIT_DEG_ROUND_SEC))) {
    *dsecfr = ddeg * 3600 - *isec;
  } else {
    *dsecfr = *isec;  // is rounded, no fractional seconds
  }
}

void CALL_CONV swe_split_deg_2_10_03(double ddeg, int32 roundflag, int32 *ideg, int32 *imin, int32 *isec, double *dsecfr, int32 *isgn)
{
  double dadd = 0;
  *isgn = 1;
  if (ddeg < 0) {
    *isgn = -1;
    ddeg = -ddeg;
  } else if (roundflag & SE_SPLIT_DEG_NAKSHATRA) {
    split_deg_nakshatra_2_10_03(ddeg, roundflag, ideg, imin, isec, dsecfr, isgn);
    return;
  }
  if (roundflag & SE_SPLIT_DEG_ROUND_DEG) {
    dadd = 0.5;
  } else if (roundflag & SE_SPLIT_DEG_ROUND_MIN) {
    dadd = 0.5 / 60.0;
  } else if (roundflag & SE_SPLIT_DEG_ROUND_SEC) {
    dadd = 0.5 / 3600.0;
  }
  if (roundflag & SE_SPLIT_DEG_KEEP_DEG) {
    if ((int32) (ddeg + dadd) - (int32) ddeg > 0)
      dadd = 0;
  } else if (roundflag & SE_SPLIT_DEG_KEEP_SIGN) {
    if (fmod(ddeg, 30) + dadd >= 30)
      dadd = 0;
  }
  ddeg += dadd;
  if (roundflag & SE_SPLIT_DEG_ZODIACAL) {
    *isgn = (int32) (ddeg / 30);
    if (*isgn == 12) // 360° = 0°
      *isgn = 0;
    ddeg = fmod(ddeg, 30);
  }
  *ideg = (int32) ddeg;
  ddeg -= *ideg;
  *imin = (int32) (ddeg * 60);
  ddeg -= *imin / 60.0;
  *isec = (int32) (ddeg * 3600);
  if (!(roundflag & (SE_SPLIT_DEG_ROUND_DEG | SE_SPLIT_DEG_ROUND_MIN | SE_SPLIT_DEG_ROUND_SEC))) {
    *dsecfr = ddeg * 3600 - *isec;
  } else {
    *dsecfr = *isec;  // is rounded, no fractional seconds
  }
}
