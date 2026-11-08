# Public API and composition

`nordtech` is an umbrella crate. Its `camera`, `config`, `events`, `inputs`,
`math`, `mesh`, `platform`, `render`, `scheduler`, `threading`, `time`, and
other modules directly re-export their standalone capability crates. It owns
no services, worker pools, event forwarding, or application lifecycle.

Every capability crate has no dependency on another Nord crate. The facade's
dependencies exist only to make those independent crates reachable through one
dependency. Programs can instead list individual crates when they want a
smaller dependency set.

## Migration from `nord_app`

The package name changed from `nord_app` to `nordtech`, and the former
`Application*` root facade types are no longer provided by the engine crate.
Use the module that owns the capability:

```rust
use nordtech::{camera, inputs, platform, render};

let platform = platform::Platform::initialize("my-game")?;
let window = platform.create_window(&platform::WindowConfig::new("My Game", 1280, 720))?;
let input_window = inputs::WindowHandle::new(window.value());
let camera = camera::PerspectiveCamera::new(1.0, 16.0 / 9.0, 0.1, 1000.0)?;
// The program chooses when to feed input, time, camera and scene data to rendering.
```

When two modules use similar concepts, their types remain distinct. The
consuming program owns conversion and ordering. For example, it can translate
platform keyboard events into input events, pass a frame duration to a fixed
step accumulator, or convert camera matrices to the renderer's GPU data format.

The workspace's [`nord_render_smoke` consumer](../examples/nord_render_smoke/)
shows one explicit composition of platform and renderer capabilities.
