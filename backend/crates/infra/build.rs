// The migrations are embedded into this crate by `sqlx::migrate!` at build
// time. Tell cargo the directory is an input, so adding a migration rebuilds
// the crate and the binary never runs against a database that is ahead of
// it ("migration … was previously applied but is missing").
fn main() {
    println!("cargo:rerun-if-changed=../../migrations");
}
