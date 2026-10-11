/* Public C catalogue queries; binary response stays in the caller's memory. */
#include "swephexp.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
static void text(const char *s) {
  uint32_t n=(uint32_t)strlen(s); fwrite(&n,4,1,stdout); fwrite(s,1,n,stdout);
}
int main(int argc,char **argv) {
  if(argc!=2)return 2;
  swe_set_ephe_path(argv[1]);
  char query[256]="1", serr[256]={0}; double magnitude=0;
  int32 status=swe_fixstar_mag(query,&magnitude,serr);
  fwrite(&status,4,1,stdout); text(serr);
  if(status<0)return 0; /* Extended C lookup is unsafe with no catalog. */
  for(int32 index=1;index<INT32_MAX;index++) {
    memset(query,0,sizeof(query));memset(serr,0,sizeof(serr));
    snprintf(query,sizeof(query),"%d",index);
    status=swe_fixstar2_mag(query,&magnitude,serr);
    fwrite(&status,4,1,stdout);text(query);text(serr);
    if(status<0)return 0;
    fwrite(&magnitude,sizeof(magnitude),1,stdout);
  }
  return 3;
}
