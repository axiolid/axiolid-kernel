#include <axiolid.h>
#include <stdio.h>

int main(void) {
  AxiolidVersion version = {0};
  if (axiolid_v0_4_version(&version) != AxiolidStatus_Ok) return 1;
  if (version.abi_major != 0 || version.abi_minor != 4) return 2;
  printf("axiolid native consumer: %u.%u\n", version.abi_major, version.abi_minor);
  return 0;
}
