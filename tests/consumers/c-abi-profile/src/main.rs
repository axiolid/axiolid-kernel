use axiolid_capi::{axiolid_v0_4_version, AxiolidStatus, AxiolidVersion};

fn main() {
    let mut version = AxiolidVersion::default();
    // SAFETY: `version` is aligned writable storage for one ABI POD value.
    let status = unsafe { axiolid_v0_4_version(&mut version) };
    assert_eq!(status, AxiolidStatus::Ok);
    println!("axiolid C ABI probe: {}.{}", version.abi_major, version.abi_minor);
}
