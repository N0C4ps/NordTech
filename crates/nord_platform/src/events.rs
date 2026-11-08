use crate::Timestamp;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Opaque native-window identifier owned by this platform instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowHandle(u64);

impl WindowHandle {
    /// Numeric identity for explicit conversion in the consuming program.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

struct WindowIdAllocator(AtomicU64);
impl WindowIdAllocator {
    const fn new() -> Self {
        Self(AtomicU64::new(1))
    }
    fn allocate(&self) -> Result<WindowHandle, EventError> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(WindowHandle)
            .map_err(|_| EventError::WindowHandleExhausted)
    }
}

/// Application lifecycle events normalized across operating systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplicationLifecycleEvent {
    Suspended,
    Resumed,
    QuitRequested,
}

/// Window events normalized across operating systems.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    Resized { width: u32, height: u32 },
    ScaleFactorChanged { scale_factor: f64 },
    FocusChanged { focused: bool },
    CloseRequested,
}

/// Normalized keyboard and pointer event emitted by native input backends.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InputEvent {
    Key { window: WindowHandle, key: KeyCode, pressed: bool, repeat: bool, modifiers: Modifiers },
    PointerMoved { window: WindowHandle, x: f64, y: f64 },
    PointerDelta { window: WindowHandle, delta_x: f64, delta_y: f64 },
    PointerButton { window: WindowHandle, button: PointerButton, pressed: bool },
    PointerWheel { window: WindowHandle, delta_x: f64, delta_y: f64 },
}

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);
impl Modifiers {
    pub const EMPTY: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const CONTROL: Self = Self(2);
    pub const ALT: Self = Self(4);
    pub const SUPER: Self = Self(8);
    #[must_use]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !0x0f == 0 { Some(Self(bits)) } else { None }
    }
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[must_use]
    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Auxiliary(u8),
}

const EVENT_QUEUE_CAPACITY: usize = 4096;

/// Normalized event with a monotonic timestamp and queue sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct PlatformEvent {
    sequence: u64,
    timestamp: Timestamp,
    kind: PlatformEventKind,
}

impl PlatformEvent {
    /// Monotonic queue sequence number.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Timestamp captured when the event entered the queue.
    #[must_use]
    pub const fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Normalized event payload.
    #[must_use]
    pub const fn kind(&self) -> &PlatformEventKind {
        &self.kind
    }
}

/// Platform-neutral event categories.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PlatformEventKind {
    /// Application lifecycle notification.
    Lifecycle(ApplicationLifecycleEvent),
    /// Normalized native window event, produced by `nord_window` backends.
    Window { window: WindowHandle, event: WindowEvent },
    /// Normalized keyboard/pointer event, produced by native input backends.
    Input(InputEvent),
}

/// Event delivery failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventError {
    /// The bounded queue is full; producer should retry or handle backpressure.
    QueueFull,
    /// Window identifier space has been exhausted.
    WindowHandleExhausted,
    /// Event sequence space has been exhausted.
    SequenceExhausted,
}

impl std::fmt::Display for EventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => formatter.write_str("platform event queue is full"),
            Self::WindowHandleExhausted => formatter.write_str("window handle space is exhausted"),
            Self::SequenceExhausted => formatter.write_str("platform event sequence is exhausted"),
        }
    }
}

impl std::error::Error for EventError {}

struct QueueState {
    events: VecDeque<PlatformEvent>,
    next_sequence: u64,
}

struct SharedQueue {
    state: Mutex<QueueState>,
    clock: crate::MonotonicClock,
    window_ids: WindowIdAllocator,
}

/// Cloneable producer for native subsystem and application events.
#[derive(Clone)]
pub struct EventEmitter {
    shared: Arc<SharedQueue>,
}

impl EventEmitter {
    /// Queue a normalized event, preserving producer order under contention.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::QueueFull`] when all bounded queue slots are in
    /// use, or [`EventError::SequenceExhausted`] if event ordering cannot be
    /// represented uniquely.
    pub fn emit(&self, kind: PlatformEventKind) -> Result<u64, EventError> {
        let mut state = self.shared.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.events.len() >= EVENT_QUEUE_CAPACITY {
            return Err(EventError::QueueFull);
        }

        let sequence = state.next_sequence;
        let next_sequence = sequence.checked_add(1).ok_or(EventError::SequenceExhausted)?;
        let timestamp = self.shared.clock.now();
        state.events.push_back(PlatformEvent { sequence, timestamp, kind });
        state.next_sequence = next_sequence;
        Ok(sequence)
    }

    /// Allocate an opaque window identifier for a native window backend.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::WindowHandleExhausted`] if the identifier space
    /// can no longer provide a unique value.
    pub fn allocate_window_handle(&self) -> Result<WindowHandle, EventError> {
        self.shared.window_ids.allocate()
    }
}

/// Event consumer owned by the platform object.
pub struct EventReceiver {
    shared: Arc<SharedQueue>,
}

impl EventReceiver {
    pub(super) fn drain_into(&self, output: &mut Vec<PlatformEvent>) {
        let mut state = self.shared.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        output.extend(state.events.drain(..));
    }
}

pub(super) fn event_queue(clock: crate::MonotonicClock) -> (EventEmitter, EventReceiver) {
    let shared = Arc::new(SharedQueue {
        state: Mutex::new(QueueState {
            events: VecDeque::with_capacity(EVENT_QUEUE_CAPACITY),
            next_sequence: 0,
        }),
        clock,
        window_ids: WindowIdAllocator::new(),
    });
    (EventEmitter { shared: Arc::clone(&shared) }, EventReceiver { shared })
}

#[cfg(test)]
mod tests {
    use super::{ApplicationLifecycleEvent, PlatformEventKind, event_queue};
    use crate::MonotonicClock;

    #[test]
    fn platform_events_keep_sequence_and_window_ids_local_to_platform() {
        let (emitter, receiver) = event_queue(MonotonicClock::new());
        let first_window = emitter.allocate_window_handle().unwrap();
        let second_window = emitter.allocate_window_handle().unwrap();
        assert_ne!(first_window, second_window);
        assert_eq!(first_window.value(), 1);
        assert_eq!(
            emitter.emit(PlatformEventKind::Lifecycle(ApplicationLifecycleEvent::Resumed)).unwrap(),
            0
        );
        assert_eq!(
            emitter
                .emit(PlatformEventKind::Lifecycle(ApplicationLifecycleEvent::Suspended))
                .unwrap(),
            1
        );

        let mut events = Vec::new();
        receiver.drain_into(&mut events);
        assert_eq!(events.iter().map(super::PlatformEvent::sequence).collect::<Vec<_>>(), [0, 1]);
        assert!(matches!(
            events[0].kind(),
            PlatformEventKind::Lifecycle(ApplicationLifecycleEvent::Resumed)
        ));
    }
}
