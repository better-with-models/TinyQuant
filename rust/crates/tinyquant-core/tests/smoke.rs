//! Smoke test — confirms tinyquant-core compiles and links.
#[test]
fn core_crate_builds() {
    // size_of::<u8>() confirms the crate links against core.
    assert_eq!(core::mem::size_of::<u8>(), 1);
}
