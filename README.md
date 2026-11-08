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

`nordtech` is the stable, general-purpose public API facade. Each capability is
also available from its own crate, so a consuming program can choose the crates
it needs and compose them explicitly. Crates should not start or coordinate
other engine capabilities implicitly; cross-crate interaction belongs in the
consuming program.
