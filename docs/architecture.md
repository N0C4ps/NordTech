# Nord Tech Architecture

## Isolated capabilities

Each crate owns one reusable capability and exposes it through its own public
API. A crate must not start, configure, or coordinate another capability as a
side effect of being used. Its behavior should depend only on its inputs and
its own state.

The consuming program chooses which crates to use and owns the code that
connects them. For example, it can translate platform events into input
actions, pass timing information to a simulation step, or turn world data into
render commands. Those relationships belong to the program because it knows
which capabilities it wants to combine.

Crate dependencies should stay narrow and explicit. Sharing a type or calling
another crate must be visible in the API and selected by the consumer; a crate
must not reach into another crate's private implementation or implicitly
subscribe to its lifecycle. A capability should remain useful when the
consumer does not use any of the other Nord Tech crates.

## Public API

`nordtech` is the stable, general-purpose public API facade for consumers that
want one dependency. The individual crates remain available for programs that
want to depend on only selected capabilities. The facade should preserve a
clear boundary: its public API may present stable names and types, while the
consuming program remains responsible for application-specific composition.

The public API must not make every consumer use every capability. APIs should
accept the values and services they need rather than retrieving unrelated
global services or discovering other crates implicitly.

## Adding or changing a crate

Before adding code, decide whether it belongs to one reusable capability or
to a consuming program's integration layer. Keep reusable behavior in its
own crate. Keep game rules, policy, and cross-capability orchestration in the
program that uses the crates.

When a public API changes, update the facade, examples, documentation, and
consumers in the same change. Prefer changes that let a crate evolve internally
without forcing unrelated crates to change.
