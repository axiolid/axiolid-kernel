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
      link: https://github.com/axiolid/kernel

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

```mermaid
flowchart LR
  accTitle: Axiolid boundary from source adapters to providers
  accDescr: Source adapters translate into neutral values and a geometry graph. Typed operation contracts carry requests to explicitly selected providers.
  Sources["Source adapters<br/>IFC, STEP, applications"] --> Neutral["Neutral values / graph<br/>Axiolid model"]
  Neutral --> Contracts["Operation contracts<br/>typed results and budgets"]
  Contracts --> Providers["Providers<br/>reference / CPU / GPU seams"]
```

That separation keeps a mesh-only user from paying for topology or GPU APIs, lets a future adapter reuse the same representation, and makes backend claims inspectable rather than implicit.

## Start with evidence

Axiolid is early software. The project deliberately distinguishes storage, contracts, and algorithms that are actually exercised. Before choosing it for an application, read:

- [Getting started](/guide/getting-started) for dependency and feature selection.
- [Capabilities](/capabilities) for what is implemented, represented, or intentionally deferred.
- [Architecture](/architecture) for dependency direction and execution seams.
- [Geometry concepts](/guide/geometry-concepts) for interactive STL examples,
  equations, tolerance, orientation, and open-profile semantics.
- [Architecture decisions](/adr/0009-layered-geometry-dag) for the non-negotiable design choices.
- [Roadmap](/ROADMAP) and [changelog](/CHANGELOG) for current direction and user-visible history.
