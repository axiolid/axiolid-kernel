//! Geometry-only traversal primitives over a layered field.
//!
//! # Scope
//!
//! This module answers geometric questions: does a route exist under an
//! explicit envelope, how long is it, and which geometric constraint blocked
//! it. It must never answer regulatory or product questions. Axiolid may say
//! "no route under this envelope"; it may not say "not wheelchair accessible", NOT-A-VERDICT
//! "ADA compliant", "valid escape route", or "rule violation". Those are NOT-A-VERDICT
//! consumer-owned interpretations of the numbers reported here.
//!
//! # Promotion status
//!
//! This is behind the non-default `navigation` feature. The house rule is that
//! a shared contract is promoted once at least two consumers need the same
//! neutral shape; until a second consumer exists, this stays opt-in so it can
//! change without breaking the default surface.

use std::collections::BinaryHeap;

use axiolid_core::Scalar;

use crate::{
    morphology::radius_in_cells, FieldChannel, FieldConfig, LayeredField, LayeredFieldError,
    PlanarMask,
};

/// Explicit geometric envelope for traversal. Every field is caller-supplied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraversalEnvelope {
    /// Lateral half-width kept free of blocking geometry, in local units.
    pub agent_radius: Scalar,
    /// Free span required above a support layer, in local units.
    pub agent_height: Scalar,
    /// Largest abrupt layer change accepted between adjacent cells.
    pub max_step: Scalar,
    /// Largest accepted rise over run between adjacent cells.
    pub max_slope: Scalar,
}

impl TraversalEnvelope {
    /// Validate the envelope. All values must be finite and non-negative.
    pub fn validate(&self) -> Result<(), LayeredFieldError> {
        let values = [
            self.agent_radius,
            self.agent_height,
            self.max_step,
            self.max_slope,
        ];
        if values.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(LayeredFieldError::InvalidEnvelope);
        }
        Ok(())
    }
}

/// A candidate support location: a cell plus the layer standing on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportNode {
    /// Cell column.
    pub x: usize,
    /// Cell row.
    pub y: usize,
    /// Layer coordinate of the supporting crossing.
    pub w: Scalar,
}

/// Structured geometric facts about a graph build and route query.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TraversalEvidence {
    /// Support nodes retained after clearance and inflation filtering.
    pub nodes: usize,
    /// Undirected adjacency pairs retained.
    pub edges: usize,
    /// Candidate supports dropped for insufficient free span above.
    pub rejected_by_height: usize,
    /// Candidate supports dropped by lateral inflation.
    pub rejected_by_radius: usize,
    /// Adjacencies dropped because the layer change exceeded `max_step`.
    pub rejected_by_step: usize,
    /// Adjacencies dropped because the gradient exceeded `max_slope`.
    pub rejected_by_slope: usize,
    /// Connected components in the retained graph.
    pub components: usize,
}

/// Result of a route query. Both variants are geometric statements.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteOutcome {
    /// A route exists under the supplied envelope.
    Route {
        /// Ordered support nodes from start to goal.
        nodes: Vec<SupportNode>,
        /// Accumulated 3D path length in local units.
        length: Scalar,
        /// Geometric facts about the search.
        evidence: TraversalEvidence,
    },
    /// No route exists under this envelope. This is not a compliance verdict. NOT-A-VERDICT
    NoRouteUnderEnvelope {
        /// Geometric facts explaining what was filtered away.
        evidence: TraversalEvidence,
    },
}

/// Traversal graph derived from a field under one explicit envelope.
#[derive(Debug, Clone)]
pub struct TraversalGraph {
    width: usize,
    cell_size: Scalar,
    nodes: Vec<Option<SupportNode>>,
    adjacency: Vec<Vec<usize>>,
    evidence: TraversalEvidence,
}

impl TraversalGraph {
    /// Build the graph from a sampled field.
    ///
    /// Support candidates are the lowest crossing in each cell. A candidate
    /// survives when the free span above it reaches `agent_height` and it is
    /// still set after the blocking mask is inflated by `agent_radius`.
    pub fn build(
        field: &LayeredField,
        config: &FieldConfig,
        envelope: &TraversalEnvelope,
    ) -> Result<Self, LayeredFieldError> {
        envelope.validate()?;
        let (width, height) = field.dimensions();
        let linear = config.tolerance().linear();
        let mut evidence = TraversalEvidence::default();

        // Lateral inflation: grow the blocking mask, then keep only cells that
        // remain outside it. Erosion of the free mask would be equivalent; the
        // inflated form is reported so the caller can inspect the obstacle set.
        let blocking = PlanarMask::from_field(field, FieldChannel::SurfacePresence).inverted();
        let inflated = blocking.dilate(config, envelope.agent_radius)?;
        let reach = radius_in_cells(config, envelope.agent_radius)?;

        let mut nodes: Vec<Option<SupportNode>> = vec![None; width * height];
        for y in 0..height {
            for x in 0..width {
                let cell = field
                    .cell(x, y)
                    .ok_or(LayeredFieldError::NodeOutsideField)?;
                let Some(support) = cell.surfaces().first() else {
                    continue;
                };
                let report = crate::clearance_above(field, config, x, y, support.w())?;
                if report.distance + linear < envelope.agent_height {
                    evidence.rejected_by_height += 1;
                    continue;
                }
                if reach > 0 && inflated.get(x, y).unwrap_or(true) {
                    evidence.rejected_by_radius += 1;
                    continue;
                }
                nodes[y * width + x] = Some(SupportNode {
                    x,
                    y,
                    w: support.w(),
                });
            }
        }
        evidence.nodes = nodes.iter().filter(|node| node.is_some()).count();

        let run = config.cell_size();
        let mut adjacency = vec![Vec::new(); nodes.len()];
        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;
                let Some(from) = nodes[index] else { continue };
                // Only forward neighbours, so each undirected pair is judged once.
                for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                    if nx >= width || ny >= height {
                        continue;
                    }
                    let neighbor = ny * width + nx;
                    let Some(to) = nodes[neighbor] else { continue };
                    let rise = (to.w - from.w).abs();
                    if rise > envelope.max_step + linear {
                        evidence.rejected_by_step += 1;
                        continue;
                    }
                    if rise / run > envelope.max_slope + linear {
                        evidence.rejected_by_slope += 1;
                        continue;
                    }
                    adjacency[index].push(neighbor);
                    adjacency[neighbor].push(index);
                    evidence.edges += 1;
                }
            }
        }

        let mut graph = Self {
            width,
            cell_size: config.cell_size(),
            nodes,
            adjacency,
            evidence,
        };
        graph.evidence.components = graph.count_components();
        Ok(graph)
    }

    /// Geometric facts about the build.
    pub const fn evidence(&self) -> TraversalEvidence {
        self.evidence
    }

    /// Support node retained at `(x, y)`, if any.
    pub fn node(&self, x: usize, y: usize) -> Option<SupportNode> {
        self.nodes.get(y * self.width + x).copied().flatten()
    }

    /// Whether two cells lie in the same connected component.
    pub fn connected(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        let (Some(start), Some(goal)) = (self.index_of(from), self.index_of(to)) else {
            return false;
        };
        self.component_labels()[start]
            .zip(self.component_labels()[goal])
            .is_some_and(|(a, b)| a == b)
    }

    /// Shortest 3D-length route under the envelope used to build this graph.
    ///
    /// Ties are broken by the lower row-major node index, so the returned path
    /// is identical across runs and platforms.
    pub fn find_route(
        &self,
        from: (usize, usize),
        to: (usize, usize),
    ) -> Result<RouteOutcome, LayeredFieldError> {
        let start = self
            .index_of(from)
            .ok_or(LayeredFieldError::NodeOutsideField)?;
        let goal = self
            .index_of(to)
            .ok_or(LayeredFieldError::NodeOutsideField)?;

        let mut best = vec![Scalar::INFINITY; self.nodes.len()];
        let mut previous = vec![usize::MAX; self.nodes.len()];
        let mut queue = BinaryHeap::new();
        best[start] = 0.0;
        queue.push(Candidate {
            cost: 0.0,
            index: start,
        });

        while let Some(Candidate { cost, index }) = queue.pop() {
            if index == goal {
                break;
            }
            if cost > best[index] {
                continue;
            }
            for neighbor in &self.adjacency[index] {
                let step = self.edge_length(index, *neighbor);
                let candidate = cost + step;
                // Strictly-better relaxation plus index tie-break keeps the
                // chosen predecessor deterministic for equal-cost paths.
                let improves = candidate < best[*neighbor]
                    || (candidate == best[*neighbor] && index < previous[*neighbor]);
                if improves {
                    best[*neighbor] = candidate;
                    previous[*neighbor] = index;
                    queue.push(Candidate {
                        cost: candidate,
                        index: *neighbor,
                    });
                }
            }
        }

        if !best[goal].is_finite() {
            return Ok(RouteOutcome::NoRouteUnderEnvelope {
                evidence: self.evidence,
            });
        }

        let mut chain = vec![goal];
        let mut cursor = goal;
        while cursor != start {
            cursor = previous[cursor];
            if cursor == usize::MAX {
                return Ok(RouteOutcome::NoRouteUnderEnvelope {
                    evidence: self.evidence,
                });
            }
            chain.push(cursor);
        }
        chain.reverse();

        Ok(RouteOutcome::Route {
            nodes: chain
                .iter()
                .map(|index| self.nodes[*index].expect("route only visits retained nodes"))
                .collect(),
            length: best[goal],
            evidence: self.evidence,
        })
    }

    fn edge_length(&self, from: usize, to: usize) -> Scalar {
        let (Some(a), Some(b)) = (self.nodes[from], self.nodes[to]) else {
            return Scalar::INFINITY;
        };
        let run = self.run_between(a, b);
        (run * run + (b.w - a.w) * (b.w - a.w)).sqrt()
    }

    fn run_between(&self, a: SupportNode, b: SupportNode) -> Scalar {
        let dx = (a.x as Scalar) - (b.x as Scalar);
        let dy = (a.y as Scalar) - (b.y as Scalar);
        (dx * dx + dy * dy).sqrt() * self.cell_run()
    }

    fn cell_run(&self) -> Scalar {
        self.cell_size
    }

    fn index_of(&self, cell: (usize, usize)) -> Option<usize> {
        let index = cell.1 * self.width + cell.0;
        (cell.0 < self.width && index < self.nodes.len() && self.nodes[index].is_some())
            .then_some(index)
    }

    fn component_labels(&self) -> Vec<Option<usize>> {
        let mut labels = vec![None; self.nodes.len()];
        let mut next = 0usize;
        let mut stack = Vec::new();
        for start in 0..self.nodes.len() {
            if self.nodes[start].is_none() || labels[start].is_some() {
                continue;
            }
            let label = next;
            next += 1;
            labels[start] = Some(label);
            stack.push(start);
            while let Some(index) = stack.pop() {
                for neighbor in &self.adjacency[index] {
                    if labels[*neighbor].is_none() {
                        labels[*neighbor] = Some(label);
                        stack.push(*neighbor);
                    }
                }
            }
        }
        labels
    }

    fn count_components(&self) -> usize {
        self.component_labels()
            .iter()
            .filter_map(|label| *label)
            .max()
            .map_or(0, |max| max + 1)
    }
}

struct Candidate {
    cost: Scalar,
    index: usize,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.index == other.index
    }
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Min-heap on cost; lower index wins ties so pops are deterministic.
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.index.cmp(&self.index))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
