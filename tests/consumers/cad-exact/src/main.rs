use axiolid_brep::ExactBRep;
use axiolid_nurbs as _;
use axiolid_topology as _;

fn main() {
    let _ = core::mem::size_of::<ExactBRep>();
    println!("cad-exact ok");
}
