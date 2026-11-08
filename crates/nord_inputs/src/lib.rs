//! Normalized keyboard and pointer events plus per-window input state.

use std::collections::BTreeMap;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, RwLock, Weak};

/// Opaque window identifier used by input state. Map platform handles to this
/// identifier in the consuming program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowHandle(u64);

impl WindowHandle {
    /// Create an input-window identity from an application-owned numeric ID.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the underlying numeric ID.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

type Listener = Arc<dyn Fn(&InputEvent) + Send + Sync + 'static>;
struct EventState {
    listeners: BTreeMap<u64, Listener>,
    next_id: u64,
}
struct EventHub {
    state: RwLock<EventState>,
}

/// RAII subscription removed automatically when dropped.
pub struct InputSubscription {
    id: u64,
    hub: Weak<EventHub>,
}

impl Drop for InputSubscription {
    fn drop(&mut self) {
        if let Some(hub) = self.hub.upgrade() {
            hub.state
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .listeners
                .remove(&self.id);
        }
    }
}

/// Input subscription setup failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSystemError;

impl Display for InputSystemError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("input listener identifier space is exhausted")
    }
}
impl Error for InputSystemError {}

/// Summary of one event delivery pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DispatchReport {
    listeners_invoked: usize,
    listeners_panicked: usize,
}
impl DispatchReport {
    #[must_use]
    pub const fn listeners_invoked(self) -> usize {
        self.listeners_invoked
    }
    #[must_use]
    pub const fn listeners_panicked(self) -> usize {
        self.listeners_panicked
    }
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        Self {
            listeners_invoked: self.listeners_invoked.saturating_add(other.listeners_invoked),
            listeners_panicked: self.listeners_panicked.saturating_add(other.listeners_panicked),
        }
    }
}

/// Platform-neutral keyboard or pointer event.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InputEvent {
    /// A key changed state. Repeated key-down messages keep `pressed = true` and set `repeat`.
    Key { window: WindowHandle, key: KeyCode, pressed: bool, repeat: bool, modifiers: Modifiers },
    /// Pointer moved in client-area physical pixels.
    PointerMoved { window: WindowHandle, x: f64, y: f64 },
    /// Relative pointer movement in device units, for captured pointer modes.
    PointerDelta { window: WindowHandle, delta_x: f64, delta_y: f64 },
    /// A pointer button changed state.
    PointerButton { window: WindowHandle, button: PointerButton, pressed: bool },
    /// A pointer wheel delta. One whole unit equals one standard wheel detent.
    PointerWheel { window: WindowHandle, delta_x: f64, delta_y: f64 },
}

impl InputEvent {
    /// Window that produced this event.
    #[must_use]
    pub const fn window(&self) -> WindowHandle {
        match self {
            Self::Key { window, .. }
            | Self::PointerMoved { window, .. }
            | Self::PointerDelta { window, .. }
            | Self::PointerButton { window, .. }
            | Self::PointerWheel { window, .. } => *window,
        }
    }
}

/// Normalized virtual key used for gameplay bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum KeyCode {
    Unknown,
    Escape,
    Enter,
    Space,
    Tab,
    Backspace,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Digit(u8),
    Letter(char),
    Function(u8),
}

/// Modifier state captured with key transitions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No active modifier keys.
    pub const EMPTY: Self = Self(0);
    /// Shift is active.
    pub const SHIFT: Self = Self(1 << 0);
    /// Control is active.
    pub const CONTROL: Self = Self(1 << 1);
    /// Alt/Option is active.
    pub const ALT: Self = Self(1 << 2);
    /// Super/Command/Windows is active.
    pub const SUPER: Self = Self(1 << 3);

    /// Build modifier flags, rejecting unknown bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !0b1111 == 0 { Some(Self(bits)) } else { None }
    }
    /// Return the underlying flags.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
    /// Combine modifier flags.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Test whether a modifier is active.
    #[must_use]
    pub const fn contains(self, modifier: Self) -> bool {
        self.0 & modifier.0 == modifier.0
    }
}

/// Normalized mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Auxiliary(u8),
}

#[derive(Default)]
struct WindowState {
    keys: BTreeSet<KeyCode>,
    buttons: BTreeSet<PointerButton>,
    pointer: Option<(f64, f64)>,
}

/// Per-window, edge-updated input state and input event dispatcher.
///
/// Native events are ingested on the application thread. State queries are
/// synchronized, and listener callbacks run after the state lock is released.
pub struct InputSystem {
    state: Mutex<HashMap<WindowHandle, WindowState>>,
    events: Arc<EventHub>,
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSystem {
    /// Create an empty input state and dispatcher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
            events: Arc::new(EventHub {
                state: RwLock::new(EventState { listeners: BTreeMap::new(), next_id: 1 }),
            }),
        }
    }

    /// Subscribe to normalized input events. Drop the returned token to unsubscribe.
    ///
    /// # Errors
    ///
    /// Returns [`InputSystemError`] if the listener identifier space is exhausted.
    pub fn subscribe<F>(&self, callback: F) -> Result<InputSubscription, InputSystemError>
    where
        F: Fn(&InputEvent) + Send + Sync + 'static,
    {
        let mut state =
            self.events.state.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or(InputSystemError)?;
        state.listeners.insert(id, Arc::new(callback));
        Ok(InputSubscription { id, hub: Arc::downgrade(&self.events) })
    }

    /// Update held state and dispatch one native input event.
    pub fn ingest(&self, event: &InputEvent) -> DispatchReport {
        {
            let mut states = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let window = match &event {
                InputEvent::Key { window, .. }
                | InputEvent::PointerMoved { window, .. }
                | InputEvent::PointerDelta { window, .. }
                | InputEvent::PointerButton { window, .. }
                | InputEvent::PointerWheel { window, .. } => *window,
            };
            let state = states.entry(window).or_default();
            match &event {
                InputEvent::Key { key, pressed: true, repeat: false, .. } => {
                    state.keys.insert(*key);
                }
                InputEvent::Key { key, pressed: false, .. } => {
                    state.keys.remove(key);
                }
                InputEvent::PointerMoved { x, y, .. } => state.pointer = Some((*x, *y)),
                InputEvent::PointerButton { button, pressed: true, .. } => {
                    state.buttons.insert(*button);
                }
                InputEvent::PointerButton { button, pressed: false, .. } => {
                    state.buttons.remove(button);
                }
                _ => {}
            }
        }
        self.dispatch(event)
    }

    /// Release all held keys and pointer buttons for a window, normally on focus loss.
    /// Synthetic release events prevent input from remaining stuck after deactivation.
    pub fn release_window(&self, window: WindowHandle) -> DispatchReport {
        let (keys, buttons) = {
            let mut states = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            states.remove(&window).map_or_else(
                || (Vec::new(), Vec::new()),
                |state| {
                    (
                        state.keys.into_iter().collect::<Vec<_>>(),
                        state.buttons.into_iter().collect::<Vec<_>>(),
                    )
                },
            )
        };
        let mut total = DispatchReport::default();
        for key in keys {
            total = total.merge(self.dispatch(&InputEvent::Key {
                window,
                key,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::EMPTY,
            }));
        }
        for button in buttons {
            total = total.merge(self.dispatch(&InputEvent::PointerButton {
                window,
                button,
                pressed: false,
            }));
        }
        total
    }

    /// Whether the given key is currently held in a window.
    #[must_use]
    pub fn is_key_pressed(&self, window: WindowHandle, key: KeyCode) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&window)
            .is_some_and(|state| state.keys.contains(&key))
    }

    /// Whether the given pointer button is currently held in a window.
    #[must_use]
    pub fn is_pointer_button_pressed(&self, window: WindowHandle, button: PointerButton) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&window)
            .is_some_and(|state| state.buttons.contains(&button))
    }

    /// Most recent pointer position in client-area pixels, if the pointer moved there.
    #[must_use]
    pub fn pointer_position(&self, window: WindowHandle) -> Option<(f64, f64)> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&window)
            .and_then(|state| state.pointer)
    }

    fn dispatch(&self, event: &InputEvent) -> DispatchReport {
        let listeners = self
            .events
            .state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .listeners
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut report = DispatchReport::default();
        for listener in &listeners {
            report.listeners_invoked = report.listeners_invoked.saturating_add(1);
            if catch_unwind(AssertUnwindSafe(|| listener(event))).is_err() {
                report.listeners_panicked = report.listeners_panicked.saturating_add(1);
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::{InputEvent, InputSystem, KeyCode, Modifiers, PointerButton, WindowHandle};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn tracks_keys_and_releases_state_per_window() {
        let system = InputSystem::new();
        let window = WindowHandle::new(7);
        let events = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&events);
        let _subscription = system
            .subscribe(move |event| {
                observed.lock().unwrap().push(event.clone());
            })
            .unwrap();

        system.ingest(&InputEvent::Key {
            window,
            key: KeyCode::Escape,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::EMPTY,
        });
        assert!(system.is_key_pressed(window, KeyCode::Escape));
        let report = system.release_window(window);
        assert_eq!(report.listeners_invoked(), 1);
        assert!(!system.is_key_pressed(window, KeyCode::Escape));
        assert_eq!(events.lock().unwrap().len(), 2);
    }

    #[test]
    fn input_listener_panics_are_counted_without_stopping_delivery() {
        let system = InputSystem::new();
        let delivered = Arc::new(AtomicUsize::new(0));
        let _panicking = system.subscribe(|_| panic!("listener failure")).unwrap();
        let counter = Arc::clone(&delivered);
        let _healthy = system
            .subscribe(move |_| {
                counter.fetch_add(1, Ordering::Relaxed);
            })
            .unwrap();
        let report = system.ingest(&InputEvent::PointerButton {
            window: WindowHandle::new(2),
            button: PointerButton::Primary,
            pressed: true,
        });
        assert_eq!(report.listeners_invoked(), 2);
        assert_eq!(report.listeners_panicked(), 1);
        assert_eq!(delivered.load(Ordering::Relaxed), 1);
    }
}
