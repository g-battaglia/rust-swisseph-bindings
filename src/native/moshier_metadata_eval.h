/* SWISSEPH
   Moshier planet routines

   modified for SWISSEPH by Dieter Koch

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

/*
 * Metadata-only specialization of swi_moshplan2 in pinned swemplan.c.
 * Derived from the Swiss Ephemeris implementation by Astrodienst AG and
 * the Moshier planetary series; the original copyright and AGPL notices
 * remain in the generated translation unit and bundled native sources.
 *
 * The generator proves that every periodic term has nonzero harmonics.
 * Immutable argument decoding and iteration over independent channels change.
 * Coefficient streams, floating expressions and Horner steps within each
 * channel retain their original order, as do the per-term accumulations.
 */

/* The caller supplies the byte representation of the WHOLE ss/cc array.
 * An index is proved to be 0..215 by the generator. Its byte offset remains
 * inside that object and identifies an aligned, existing double subobject.
 * Starting from ss[0] instead would not permit crossing a row boundary. */
static inline double swe_moshier_harmonic_value(const unsigned char *object,
                                               unsigned char index)
{
  return *(const double *)(object + sizeof(double) * index);
}

int swi_moshplan2 (double J, int iplm, double *pobj)
{
  int i, j, ip, nt;
  unsigned int term_index;
  double *pl, *pb, *pr;
  double su, cu, sv, cv, T;
  double t, sl, sb, sr;
  const struct plantbl *plan = planets[iplm];
  const struct swe_moshier_plan *metadata = &swe_moshier_plans[iplm];
  const struct swe_moshier_argument *argument = metadata->arguments;

  T = (J - J2000) / TIMESCALE;
  /* Calculate sin( i*MM ), etc. for needed multiple angles. */
  for (i = 0; i < 9; i++)
    {
      if ((j = plan->max_harmonic[i]) > 0)
        {
          sr = (mods3600 (freqs[i] * T) + phases[i]) * STR;
          sscc (i, sr, j);
        }
    }

  pl = plan->lon_tbl;
  pb = plan->lat_tbl;
  pr = plan->rad_tbl;
  sl = 0.0;
  sb = 0.0;
  sr = 0.0;

  for (term_index = 0; term_index < metadata->count; term_index++)
    {
      const struct swe_moshier_term *term = &metadata->terms[term_index];
      nt = term->degree;
      if (term->polynomial)
        {
          double longitude = *pl++;
          double latitude = *pb++;
          double radius = *pr++;
          /* Each immutable coefficient stream has the same degree. Keep its
           * Horner order, while exposing the independent channels together. */
          for (ip = 0; ip < nt; ip++)
            {
              longitude = longitude * T + *pl++;
              latitude = latitude * T + *pb++;
              radius = radius * T + *pr++;
            }
          sl += mods3600 (longitude);
          sb += latitude;
          sr += radius;
          continue;
        }

      /* Assign the first angle, without multiplying by an identity. Negate
       * the sine exactly as C does, including a possible negative zero. */
      su = swe_moshier_harmonic_value((const unsigned char *)&ss, argument->index);
      if (argument->negative)
        su = -su;
      cu = swe_moshier_harmonic_value((const unsigned char *)&cc, argument->index);
      sv = su;
      cv = cu;
      argument++;
      for (ip = 1; ip < term->count; ip++, argument++)
        {
          su = swe_moshier_harmonic_value((const unsigned char *)&ss, argument->index);
          if (argument->negative)
            su = -su;
          cu = swe_moshier_harmonic_value((const unsigned char *)&cc, argument->index);
          t = su * cv + cu * sv;
          cv = cu * cv - su * sv;
          sv = t;
        }

      {
        double longitude_cos = *pl++;
        double longitude_sin = *pl++;
        double latitude_cos = *pb++;
        double latitude_sin = *pb++;
        double radius_cos = *pr++;
        double radius_sin = *pr++;
        /* No horizontal reduction: every channel retains its own Horner
         * recurrence, then contributes to its original ordered accumulator. */
        for (ip = 0; ip < nt; ip++)
          {
            longitude_cos = longitude_cos * T + *pl++;
            longitude_sin = longitude_sin * T + *pl++;
            latitude_cos = latitude_cos * T + *pb++;
            latitude_sin = latitude_sin * T + *pb++;
            radius_cos = radius_cos * T + *pr++;
            radius_sin = radius_sin * T + *pr++;
          }
        sl += longitude_cos * cv + longitude_sin * sv;
        sb += latitude_cos * cv + latitude_sin * sv;
        sr += radius_cos * cv + radius_sin * sv;
      }
    }
  pobj[0] = STR * sl;
  pobj[1] = STR * sb;
  pobj[2] = STR * plan->distance * sr + plan->distance;
  return OK;
}
