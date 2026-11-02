//! Regenerates `COMMANDS.md`, the command reference installed with the `trama-design` skill, from
//! the command registry: `cargo run -p trama-cli --example write_command_reference`.
//!
//! The file is embedded in `trama-core` at build time, so build again after running this; a test
//! of `trama-cli` fails until the embedded copy equals what the registry renders.

fn main() {
    let path = trama_cli::registry::command_reference_source();
    let text = trama_cli::registry::command_reference();
    if let Err(error) = std::fs::write(&path, text) {
        eprintln!("cannot write {}: {error}", path.display());
        std::process::exit(1);
    }
    println!("wrote {}", path.display());
}
