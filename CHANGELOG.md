# Changelog

All notable changes to Nord Tech will be documented here.

## Unreleased

- Rename the public facade crate from `nord_app` to `nordtech`.
- Make `nordtech` a module-only facade and move app-level integration into the render smoke consumer.
- Remove direct Nord crate dependencies between capability crates.
- Give camera, mesh, input, platform, jobs, scheduler, and renderer standalone contracts.
- Initial standalone engine workspace.
