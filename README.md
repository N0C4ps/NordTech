# Nord Tech

Nord Tech is a collection of reusable Rust engine and infrastructure crates.
The workspace is independent from any game project; game code, assets, and
shaders belong to each consuming game.

## Requirements

- Rust toolchain pinned in `rust-toolchain.toml` (install through rustup)
- Windows MSVC build tools for native Windows dependencies

## Workspace commands

Run from the repository root:

```powershell
cargo check --workspace --locked
cargo fmt --all -- --check
cargo lint
cargo build --workspace --locked
```

`nord_app` is the application-facing facade. Other crates can be added directly
when a project needs their APIs. Internal crate dependencies use workspace-local
paths and resolve as part of this workspace.
