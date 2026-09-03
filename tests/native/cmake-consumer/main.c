#include <axiolid.h>
#include <stdint.h>
#include <stdio.h>

#define CHECK(call)                                                            \
  do {                                                                         \
    AxiolidStatus status = (call);                                              \
    if (status != AxiolidStatus_Ok) {                                           \
      fprintf(stderr, "line %d: status %d\n", __LINE__, (int)status);          \
      return 1;                                                                \
    }                                                                          \
  } while (0)

static const double TRIANGLE_POSITIONS[] = {0, 0, 0, 1, 0, 0, 0, 1, 0};
static const uint32_t TRIANGLE_INDICES[] = {0, 1, 2};

int main(void) {
  AxiolidVersion version = {0};
  CHECK(axiolid_v0_4_version(&version));
  if (version.abi_major != 0 || version.abi_minor != 4) return 2;

  AxiolidContextConfig config = {AXIOLID_PROVIDER_PORTABLE, 8, 8, 64, 64};
  AxiolidContextHandle context = AxiolidContextHandle_INVALID;
  CHECK(axiolid_v0_4_context_create(&config, &context));

  AxiolidMeshHandle mesh = AxiolidMeshHandle_INVALID;
  CHECK(axiolid_v0_4_mesh_import(context, TRIANGLE_POSITIONS, 3,
                                 TRIANGLE_INDICES, 1, &mesh));

  AxiolidTolerance tolerance = {1e-9, 1e-12};
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
  AxiolidErrorInfo error = {0};
  CHECK(axiolid_v0_4_context_last_error_info(context, &error));
  if (error.status != AxiolidStatus_UnsupportedExact) return 5;

  CHECK(axiolid_v0_4_result_destroy(context, exact));
  CHECK(axiolid_v0_4_mesh_destroy(context, mesh));
  CHECK(axiolid_v0_4_context_destroy(context));
  printf("axiolid native consumer: %u.%u success + typed refusal\n",
         version.abi_major, version.abi_minor);
  return 0;
}
