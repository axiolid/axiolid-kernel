#include "axiolid.h"

#include <cstdio>

int main() {
  AxiolidVersion version{};
  if (axiolid_v0_4_version(&version) != AxiolidStatus_Ok) {
    return 1;
  }
  std::printf("axiolid native C++ consumer: %u.%u\n", version.abi_major, version.abi_minor);
  return version.abi_major == 0 && version.abi_minor == 4 ? 0 : 2;
}
