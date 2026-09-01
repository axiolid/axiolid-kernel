# axiolid-nurbs

L2 format-neutral algorithms over polynomial and rational B-spline values.

- Reuse `axiolid-scalar` as the correctness oracle; do not duplicate evaluation.
- Every tolerance-sensitive solver receives explicit bounded options.
- Shape-preserving transforms must be verified by independent evaluation samples.
- Closed metadata is not proof of periodicity; wrapping requires an explicit seam check.
- No importer, tessellator, file-format, or vendor vocabulary belongs here.
