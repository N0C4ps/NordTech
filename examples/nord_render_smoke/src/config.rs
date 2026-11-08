//! Stable application-facing facade over Nord's configuration loader.

pub use nordtech::config::{
    ConfigError as ApplicationConfigError, ConfigFileFormat as ApplicationConfigFileFormat,
    ConfigLoader as ApplicationConfigLoader, ConfigParseError as ApplicationConfigParseError,
    ConfigValidator as ApplicationConfigValidator,
};
