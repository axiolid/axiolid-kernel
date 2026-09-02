use axiolid_core::{Point3, Vec3};
use axiolid_curve::Line3;
use axiolid_evaluate::evaluate3;

fn main() {
    let line = Line3 { origin: Point3::new(0.0, 0.0, 0.0), direction: Vec3::new(1.0, 0.0, 0.0) };
    let p = evaluate3(&axiolid_curve::Curve3::Line(line), 2.0).expect("evaluation");
    assert!((p.x - 2.0).abs() < 1e-12);
    println!("parametric-curves ok");
}
