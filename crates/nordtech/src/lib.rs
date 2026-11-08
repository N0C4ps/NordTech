//! Public module facade for Nord Tech's independent capability crates.
//!
//! This crate only makes the individual libraries available under one
//! dependency. It does not construct services or connect their lifecycles.
//! Programs choose the modules they need and own any integration between them.

pub use nord_camera as camera;
pub use nord_config as config;
pub use nord_crash as crash;
pub use nord_dependency as dependency;
pub use nord_diagnostics as diagnostics;
pub use nord_events as events;
pub use nord_generation as generation;
pub use nord_hash as hash;
pub use nord_inputs as inputs;
pub use nord_jobs as jobs;
pub use nord_json_compressor as json_compressor;
pub use nord_log as log;
pub use nord_math as math;
pub use nord_memory as memory;
pub use nord_mesh as mesh;
pub use nord_physics as physics;
pub use nord_platform as platform;
pub use nord_render as render;
pub use nord_scheduler as scheduler;
pub use nord_threading as threading;
pub use nord_time as time;
pub use nord_world as world;
