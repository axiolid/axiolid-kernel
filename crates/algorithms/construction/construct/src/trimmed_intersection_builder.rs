use axiolid_brep::{Curve2Id, ExactBRep, ExactBRepBuilder, SurfaceId};
use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Interval, Point2, Point3};
use axiolid_curve::{Curve2, Curve3, Line2, Line3};
use axiolid_surface::Surface;
use axiolid_topology::{
    Edge, EdgeId, EdgeUse, Face, FaceBound, FaceId, Loop, LoopId, Orientation, Vertex, VertexId,
};

use crate::BACKEND_ID;

pub(super) struct ArrangementBuilder {
    inner: ExactBRepBuilder,
}

impl ArrangementBuilder {
    pub fn new() -> GeomResult<Self> {
        let mut inner = ExactBRepBuilder::default();
        inner
            .try_reserve(11, 13, 2, 11, 12)
            .map_err(|_| allocation_error("trimmed B-rep support allocation"))?;
        inner
            .topology_mut()
            .try_reserve(10, 11, 3, 3, 0, 0)
            .map_err(|_| allocation_error("trimmed B-rep topology allocation"))?;
        Ok(Self { inner })
    }

    pub fn add_surface(&mut self, surface: Surface) -> SurfaceId {
        self.inner.add_surface(surface)
    }

    pub fn add_vertex(&mut self, position: Point3) -> VertexId {
        self.inner.topology_mut().add_vertex(Vertex { position })
    }

    pub fn add_line_edge(
        &mut self,
        start: VertexId,
        end: VertexId,
        start_point: Point3,
        end_point: Point3,
    ) -> GeomResult<EdgeId> {
        let direction = end_point - start_point;
        if !start_point.is_finite()
            || !end_point.is_finite()
            || !direction.is_finite()
            || direction.length_squared() == 0.0
        {
            return Err(GeomError::Degenerate(
                "trimmed B-rep edge has a non-finite or zero-length carrier".into(),
            ));
        }
        let curve = self.inner.add_curve3(Curve3::Line(Line3 {
            origin: start_point,
            direction,
        }));
        let edge = self.inner.topology_mut().add_edge(Edge {
            start,
            end,
            curve: Some(curve),
        });
        self.inner.set_edge_interval(edge, Interval::UNIT);
        Ok(edge)
    }

    pub fn add_pcurve(&mut self, start: Point2, end: Point2) -> GeomResult<Curve2Id> {
        let direction = end - start;
        if !start.is_finite()
            || !end.is_finite()
            || !direction.is_finite()
            || direction.length_squared() == 0.0
        {
            return Err(GeomError::Degenerate(
                "trimmed B-rep pcurve has a non-finite or zero-length carrier".into(),
            ));
        }
        Ok(self.inner.add_curve2(Curve2::Line(Line2 {
            origin: start,
            direction,
        })))
    }

    pub fn add_loop(&mut self, edges: Vec<EdgeUse<Curve2Id>>) -> GeomResult<LoopId> {
        if edges.len() < 3 {
            return Err(GeomError::Degenerate(
                "trimmed B-rep loop needs at least three edge uses".into(),
            ));
        }
        let use_count = edges.len();
        let loop_id = self.inner.topology_mut().add_loop(Loop { edges });
        for use_index in 0..use_count {
            self.inner
                .set_pcurve_interval(loop_id, use_index, Interval::UNIT);
        }
        Ok(loop_id)
    }

    pub fn add_face(&mut self, loop_id: LoopId, surface: SurfaceId) -> GeomResult<FaceId> {
        let mut bounds = Vec::new();
        bounds
            .try_reserve_exact(1)
            .map_err(|_| allocation_error("trimmed B-rep face bound allocation"))?;
        bounds.push(FaceBound {
            loop_id,
            orientation: Orientation::Forward,
            outer: true,
        });
        Ok(self.inner.topology_mut().add_face(Face {
            surface: Some(surface),
            bounds,
            orientation: Orientation::Forward,
        }))
    }

    pub fn finish(self) -> GeomResult<ExactBRep> {
        self.inner
            .finish()
            .map_err(|error| GeomError::BackendContractViolation {
                backend: BACKEND_ID,
                detail: format!("trimmed B-rep assembly failed: {error:?}"),
            })
    }
}

pub(super) fn edge_use(
    edge: EdgeId,
    orientation: Orientation,
    pcurve: Curve2Id,
) -> EdgeUse<Curve2Id> {
    EdgeUse {
        edge,
        orientation,
        pcurve: Some(pcurve),
    }
}

pub(super) fn allocation_error(resource: &'static str) -> GeomError {
    GeomError::BudgetExceeded { resource }
}
