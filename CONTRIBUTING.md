# Contributing

Before opening a pull request, run `cargo fmt --all -- --check`,
`cargo check --workspace --locked`, and `cargo lint` from the repository root.
Keep game-specific code, assets, and shaders in consuming game repositories.
Update `CHANGELOG.md` when a change affects consumers.
