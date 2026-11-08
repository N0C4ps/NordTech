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

`nordtech` is a module facade over the independent capability crates. For
example, a program can import `nordtech::platform`, `nordtech::render`, and
`nordtech::camera`; it can also depend on a single capability crate directly.
The facade only re-exports modules. Each engine crate has no dependency on
another Nord crate, and application code owns any integration between them.

The workspace includes `examples/nord_render_smoke`, a small consumer
application that composes the platform and renderer APIs explicitly.
See [Public API and composition](docs/public-api.md) for the `nord_app` to
`nordtech` migration and examples of connecting independent capabilities.
