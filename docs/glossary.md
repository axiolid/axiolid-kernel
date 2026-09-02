# Glossary

Axiolid documentation links the first prose use of each glossary term on a page.
Hover the linked term—or focus it with the keyboard—to see the concise
definition. Follow the link for the stable glossary entry.

## API {#api}

**Application programming interface.** The public types and operations through which software uses a library or service.

## B-rep {#b-rep}

**Boundary representation.** A solid or region represented by connected topological boundaries together with their supporting geometry.

## DAG {#dag}

**Directed acyclic graph.** A directed graph with no directed cycle; Axiolid uses backward-only typed references to keep its neutral geometry graph acyclic.

## IFC {#ifc}

**Industry Foundation Classes.** An open, vendor-neutral data model for built-environment information exchange; IFC interpretation stays outside Axiolid's format-neutral kernel.

## NURBS {#nurbs}

**Non-uniform rational B-spline.** A weighted parametric curve or surface representation that can express polynomial shapes and exact conics.

## predicate {#predicate}

**Geometric predicate.** A classification question—such as orientation or sidedness—whose sign or discrete outcome controls a geometric algorithm.

## provider {#provider}

**Provider.** A replaceable implementation of one or more portable Axiolid operation contracts; provider selection belongs to execution policy, not the contract.

## STL {#stl}

**Stereolithography triangle format.** A mesh interchange format that lists triangular facets; an STL mesh does not by itself prove watertightness, manifoldness, or exact geometry.

## tolerance {#tolerance}

**Tolerance.** An explicit caller-selected numerical bound used by an operation for admissible approximation or classification; Axiolid does not hide one global epsilon.

## typed refusal {#typed-refusal}

**Typed refusal.** A structured non-success result stating why an operation could not establish its contract, rather than returning an unchecked approximation or ambiguous boolean.
