# Configuration system

`nord_config` is a format-independent configuration loader, available through
`nordtech::config`. It owns file parsing and value composition only. A
consuming program defines its own schema, defaults, validation, and policy.

## Sources and precedence

The loader starts with typed defaults and then applies these layers:

1. Defaults serialized from the schema's Rust `Default` implementation.
2. JSON or TOML files, in the order they are added to the loader.
3. Process environment variables matching the configured prefix.
4. Explicit environment overrides supplied by an embedding application.

Objects/tables merge recursively. Arrays and scalar values replace the earlier
value. Missing optional files are ignored; malformed, unreadable, or unsupported
files are errors. Deserialization and application validation happen after all
sources have been merged, so consumers only receive a complete, validated
configuration.

The loader does not select a path or read process environment automatically;
the consuming program chooses its sources and precedence.

## Environment variables

Double underscores separate nested field names. For example:

```powershell
$env:NORD__APPLICATION__TITLE = 'Nord: Sandbox'
$env:NORD__WINDOW__WIDTH = '1600'
$env:NORD__WINDOW__HEIGHT = '900'
$env:NORD__MEMORY__BUDGET_BYTES = '536870912'
$env:NORD__TIMING__MAX_FRAME_DELTA_MS = '250'
$env:NORD__TIMING__TIME_SCALE = '1.0'
```

The loader treats JSON scalar syntax as typed values, so numbers, booleans,
arrays, objects, and quoted strings can be represented. A plain non-JSON value
is treated as a string. Invalid paths, incompatible merges, unknown fields,
type mismatches, and values rejected by the game schema fail startup with
source context; config values are not silently discarded.

## Formats

TOML is the default human-edited file format for the game. The engine loader
also supports JSON for tools, generated configuration, and deployments. File
format is inferred case-insensitively from `.toml` or `.json`; callers can set
the format explicitly for extensionless files. TOML and JSON deserialize into
the same Serde schema, so format choice does not change game configuration
semantics.

The loader does not impose a game schema, application directory, or runtime
clock. Configuration hot reload, profile selection, remote providers, and
secret stores belong in a consumer or in a future crate with its own explicit
API.
