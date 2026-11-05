//! Regenerates `COMMANDS.md`, the command reference installed with the `weaver-design` skill, from
//! the command registry: `cargo run -p weaver-cli --example write_command_reference`.
//!
//! The file is embedded in `weaver-core` at build time, so build again after running this; a test
//! of `weaver-cli` fails until the embedded copy equals what the registry renders.

fn main() {
    let path = weaver_cli::registry::command_reference_source();
    let text = weaver_cli::registry::command_reference();
    if let Err(error) = std::fs::write(&path, text) {
        eprintln!("cannot write {}: {error}", path.display());
        std::process::exit(1);
    }
    println!("wrote {}", path.display());
}
