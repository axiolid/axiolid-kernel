# axiolid-linear

L1 linear representation vocabulary: `Line`, `Ray2`, `Segment`, `Polyline`.

- Data only. No evaluation, no tolerance policy, no algorithms, no features.
- `Ray3` is re-exported from `axiolid-core`, never redefined — a second `Ray3`
  would split the vocabulary for existing consumers.
- Directions are stored as authored; do not normalise on construction, because
  that silently reparameterises the caller's geometry.
- `axiolid-curve` depends on this package and re-exports it. Both
  `axiolid_linear::Line2` and `axiolid_curve::Line2` must keep naming this type.
- This package exists so a line-query consumer compiles without curves,
  surfaces, meshes, topology, providers, or execution. Adding any dependency
  beyond `axiolid-core` defeats its reason to exist.
