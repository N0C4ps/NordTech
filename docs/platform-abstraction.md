# Nord Platform Abstraction Layer

## Ownership and boundaries

`nord_platform` implements native platform capabilities and is available via
`nordtech::platform`. The crate owns its path, clock, file, display, window,
and platform-event APIs. It has no dependency on `nord_events` or `nord_inputs`;
the consuming program decides whether and how to translate platform
notifications into those separate event and input systems.

## Capabilities

- **Backend identity:** the native backend and target architecture are
  selected at compile time. Windows is the primary development target.
- **Application paths:** data, config, cache, and temporary paths are resolved
  without creating directories implicitly. Call `ensure_*_dir` when needed.
- **Monotonic time:** elapsed timestamps use `std::time::Instant`.
- **Explicit file resources:** opening returns an opaque handle; reading,
  writing, seeking, flushing, and closing go through the owning filesystem.
- **Native windows:** `WindowSystem` creates top-level windows with opaque
  platform-owned handles. Native creation, message pumping, and destruction
  stay on the initializing thread. Wrong-thread calls return an error.
- **Displays:** `DisplayManager` returns fresh monitor snapshots and can find
  the monitor containing a virtual desktop point.
- **Platform event queue:** native notifications carry this crate's own window
  identity, sequence number, and monotonic timestamp. The bounded queue returns
  an error when full; the consumer drains it into a reusable buffer.

## Native backends

Backend selection uses Rust `cfg(target_os)` and native details stay in
`crates/nord_platform/src/backend/`. Windows uses a thin `windows-sys` binding
layer. Its window procedure translates close, resize, focus, DPI, keyboard,
and pointer messages into platform-owned events. Linux and macOS currently
implement path discovery only; their native window backends still need
implementations.

Use native `PathBuf` and `OsStr` values rather than assuming paths are UTF-8.
Raw OS handles remain behind the backend except for the explicit native window
handle returned to a consumer that chooses to initialize a renderer.

## Consumer-owned composition

Windows HWNDs must be created, pumped, and destroyed on their owner thread.
The platform object does not run an application loop, create directories,
open files, start jobs, or dispatch events into another crate. The consumer
chooses when to poll and how to handle each event. For example, it can convert
`PlatformEventKind::Input` into `nordtech::inputs::InputEvent` after mapping
the distinct window IDs.

The platform object does not implicitly create files or directories. Registering
the Win32 window class and creating an HWND happen only after the program
explicitly requests a window.

## Technical references

- [Rust conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
- [`std::time::Instant`](https://doc.rust-lang.org/stable/std/time/struct.Instant.html)
- [Microsoft: creating a window](https://learn.microsoft.com/en-us/windows/win32/learnwin32/creating-a-window)
- [Microsoft: writing the window procedure](https://learn.microsoft.com/en-us/windows/win32/learnwin32/writing-the-window-procedure)
- [Microsoft: setting process DPI awareness](https://learn.microsoft.com/en-us/windows/win32/hidpi/setting-the-default-dpi-awareness-for-a-process)
- [Microsoft: enumerating display monitors](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaymonitors)
- [Microsoft: retrieving monitor information](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmonitorinfow)
- [`windows-sys` Windows message APIs](https://docs.rs/windows-sys/0.61.2/windows_sys/Win32/UI/WindowsAndMessaging/index.html)
