//! `include_dir!` does not make cargo watch the embedded folder on stable Rust: this
//! script does, so that an edit of an installed text rebuilds the crate.

fn main() {
    println!("cargo:rerun-if-changed=assets");
}
