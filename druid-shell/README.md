# druid-shell

`druid-shell` is an attempt to provide a common interface to the various
elements of different platform application frameworks. It is designed to be used
by [druid], a UI toolkit.

## Design

The code in `druid-shell` can be divided into roughly two categories: the
platform agnostic code and types, which are exposed directly, and the
platform-specific implementations of these types, which live in per-platform
directories in `src/platform`. The platform-specific code for the current
platform is reexported as `druid-shell::platform`.

`druid-shell` does not generally expose platform types directly. Instead, we
expose wrapper structs that define the common interface, and then call
corresponding methods on the concrete type for the current platform.

## Unsafe

Interacting with system APIs is inherently unsafe. One of the goals of
`druid-shell` is to handle all interaction with these APIs, exposing
a safe interface to `druid` and other possible consumers.

[druid]: https://github.com/linebender/druid
