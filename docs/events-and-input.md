# Events and input

## Independent capabilities

`nord_events` defines a typed event dispatcher and its own event/window
identities. `nord_inputs` defines normalized keyboard and pointer events, held
state, and a separate input-window identity. `nord_platform` defines native
window and input notifications in its platform event queue. None of these
crates depends on the others or forwards events into another crate.

The consumer chooses which notifications matter and translates them. Window
IDs are distinct types on purpose; map them explicitly at the application
boundary:

```rust
let platform_window = platform.create_window(&config)?;
let input_window = nordtech::inputs::WindowHandle::new(platform_window.value());
let event_window = nordtech::events::WindowHandle::from_raw(platform_window.value());
```

The program can then turn a `nordtech::platform::PlatformEventKind::Input`
value into an `nordtech::inputs::InputEvent`, update the input system, and
dispatch a separate application event if its own policy requires that.

## Dispatcher behavior

Both dispatchers expose subscription tokens that unregister on drop. Callbacks
run without holding the subscriber lock. Listener panics are counted when the
build uses unwinding; a build configured with `panic = "abort"` terminates on a
panic.

`InputSystem` tracks keys, pointer buttons, and pointer position per input
window. On focus loss, the consumer can call `release_window` to clear held
state and emit synthetic releases. Platform polling and the decision to
interpret focus or input notifications remain application policy.

## Native behavior

The Windows platform backend currently normalizes focused-window key down/up,
repeat, modifiers, pointer movement, buttons, wheel messages, focus, resize,
and DPI changes. Its event queue is bounded at 4096 entries and assigns a
sequence and monotonic timestamp as each event is enqueued. Queue saturation
returns an explicit error. Linux and macOS window backends still need native
implementations.

Raw Input is not registered yet. Device identity, unaccelerated relative mouse
motion, and high-polling-rate support should be enabled by the consuming
program when it owns that process-level choice.

## References

- [Microsoft: keyboard input overview](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input)
- [Microsoft: virtual-key codes](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes)
- [Microsoft: Raw Input overview](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-raw-input)
- [Microsoft: using Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input)
- [Microsoft: `WM_INPUT`](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-input)
