---
layout: home

hero:
  name: Axiolid
  text: Geometry infrastructure without format or backend lock-in.
  tagline: A pure-Rust, format-agnostic geometry kernel with neutral data, explicit contracts, and replaceable execution providers.
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: Browse capabilities
      link: /capabilities
    - theme: alt
      text: View on GitHub
      link: https://github.com/axiolid/axiolid-kernel

features:
  - title: Neutral by construction
    details: The model owns geometry values—not IFC, STEP, renderer, or GPU API vocabulary. Adapters translate at the boundary.
  - title: Small by default
    details: Start with core values or a leaf crate. Representation, algorithms, and execution contexts are opt-in rather than a mandatory bundle.
  - title: Contracts before providers
    details: Operation contracts live apart from CPU, GPU, and third-party providers. Capability means an executable implementation, not a boolean claim.
  - title: Portable correctness
    details: Scalar reference paths are the oracle. SIMD and native acceleration must be measured and differentially verified before they earn complexity.
---

## A kernel is a boundary, not a format parser

Axiolid is the neutral middle layer for applications that import, construct, query, compile, or render geometry. It is designed so source-format semantics stay in adapters and execution choices stay in providers.

```text
source adapters ──► neutral values / graph ──► operation contracts ──► providers
     IFC, STEP             Axiolid model          Axiolid kernel      scalar / CPU / GPU
```

That separation keeps a mesh-only user from paying for topology or GPU APIs, lets a future adapter reuse the same representation, and makes backend claims inspectable rather than implicit.

## Start with evidence

Axiolid is early software. The project deliberately distinguishes storage, contracts, and algorithms that are actually exercised. Before choosing it for an application, read:

- [Getting started](/guide/getting-started) for dependency and feature selection.
- [Capabilities](/capabilities) for what is implemented, represented, or intentionally deferred.
- [Architecture](/architecture) for dependency direction and execution seams.
- [Architecture decisions](/adr/0009-layered-geometry-dag) for the non-negotiable design choices.
- [Roadmap](/ROADMAP) and [changelog](/CHANGELOG) for current direction and user-visible history.
