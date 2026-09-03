use axiolid::application::ApplicationBuilder;
use axiolid::contracts::{capability_ids, CapabilityRequirement, Exactness, ExecutionOptions, Representation};
use axiolid::core::{BooleanOperator, Point3, Ray3, Tolerance, Vec3};
use axiolid::mesh::TriMesh;
use axiolid::profile::{Profile, RectangleProfile};

fn cube(min: Point3, max: Point3) -> TriMesh {
    let p = vec![
        Point3::new(min.x,min.y,min.z), Point3::new(max.x,min.y,min.z),
        Point3::new(max.x,max.y,min.z), Point3::new(min.x,max.y,min.z),
        Point3::new(min.x,min.y,max.z), Point3::new(max.x,min.y,max.z),
        Point3::new(max.x,max.y,max.z), Point3::new(min.x,max.y,max.z),
    ];
    let i = vec![0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7];
    TriMesh::new(p, i)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = ApplicationBuilder::new()
        .with_portable_boolean()?
        .with_portable_section()?
        .build();
    let tolerance = Tolerance::new(1e-9, 1e-9).expect("finite tolerance");
    let options = ExecutionOptions::new(tolerance);
    let subject = cube(Point3::ZERO, Point3::splat(2.0));
    let tool = cube(Point3::splat(1.0), Point3::splat(3.0));

    assert!(app.validate_mesh(&subject, tolerance)?.is_closed_two_manifold());
    let measured = app.measure_mesh(&subject, tolerance)?;
    assert!(measured.surface.area > 0.0 && measured.volume.signed_volume > 0.0);
    assert!(!app.boolean(&subject, &tool, BooleanOperator::Difference, &options)?.mesh.indices.is_empty());
    assert!(!app.subtract_many(&subject, std::slice::from_ref(&tool), &options)?.mesh.indices.is_empty());

    let ray = Ray3 { origin: Point3::new(-1.0,1.0,1.0), direction: Vec3::X };
    assert!(app.nearest_mesh_hit(&subject, &ray, tolerance)?.is_some());
    let rectangle = Profile::Rectangle(RectangleProfile {
        x: 2.0, y: 3.0, thickness: None, outer_radius: None, inner_radius: None,
    });
    assert!(!app.extrude_profile_exact(&rectangle, Vec3::Z, 4.0, tolerance)?.topology().faces().is_empty());

    let need = CapabilityRequirement {
        id: capability_ids::EXACT_EXTRUDE,
        output: Representation::ExactBrep,
        exactness: Exactness::Exact,
        deterministic: true,
    };
    let exact = app.descriptor().require(need)?;
    assert_eq!(exact.provider.as_str(), "scalar-generate");
    println!("axiolid facade probe: {} capabilities", app.descriptor().capabilities.len());
    Ok(())
}
