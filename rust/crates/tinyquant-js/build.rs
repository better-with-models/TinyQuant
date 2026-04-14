//! napi-rs build-time hook. Generates the N-API symbol list required
//! for the `cdylib` to be loadable as a Node `.node` module.

fn main() {
    napi_build::setup();
}
