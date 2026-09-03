#include <axiolid.h>

#include <array>
#include <cstdint>
#include <cstdio>

#define CHECK(call)                                                            \
  do {                                                                         \
    AxiolidStatus status = (call);                                              \
    if (status != AxiolidStatus_Ok) {                                           \
      std::fprintf(stderr, "line %d: status %d\n", __LINE__,                   \
                   static_cast<int>(status));                                   \
      return 1;                                                                \
    }                                                                          \
  } while (false)

int main() {
  AxiolidVersion version{};
  CHECK(axiolid_v0_4_version(&version));
  if (version.abi_major != 0 || version.abi_minor != 4) return 2;

  AxiolidContextConfig config{AXIOLID_PROVIDER_PORTABLE, 8, 8, 64, 64};
  AxiolidContextHandle context = AxiolidContextHandle_INVALID;
  CHECK(axiolid_v0_4_context_create(&config, &context));

  constexpr std::array<double, 9> positions{0, 0, 0, 1, 0, 0, 0, 1, 0};
  constexpr std::array<std::uint32_t, 3> indices{0, 1, 2};
  AxiolidMeshHandle mesh = AxiolidMeshHandle_INVALID;
  CHECK(axiolid_v0_4_mesh_import(context, positions.data(), 3, indices.data(), 1,
                                 &mesh));

  AxiolidTolerance tolerance{1e-9, 1e-12};
  AxiolidResultHandle exact = AxiolidResultHandle_INVALID;
  CHECK(axiolid_v0_4_exact_extrude_rectangle(context, 2, 3, 4, tolerance,
                                              &exact));
  AxiolidGeometryKind kind = AxiolidGeometryKind_Unknown;
  CHECK(axiolid_v0_4_result_kind(context, exact, &kind));
  if (kind != AxiolidGeometryKind_ExactBrep) return 3;

  AxiolidResultHandle refused = AxiolidResultHandle_INVALID;
  if (axiolid_v0_4_exact_boolean(context, mesh, mesh, tolerance, &refused) !=
      AxiolidStatus_UnsupportedExact)
    return 4;
  AxiolidErrorInfo error{};
  CHECK(axiolid_v0_4_context_last_error_info(context, &error));
  if (error.status != AxiolidStatus_UnsupportedExact) return 5;

  CHECK(axiolid_v0_4_result_destroy(context, exact));
  CHECK(axiolid_v0_4_mesh_destroy(context, mesh));
  CHECK(axiolid_v0_4_context_destroy(context));
  std::printf("axiolid native C++ consumer: %u.%u success + typed refusal\n",
              version.abi_major, version.abi_minor);
  return 0;
}
