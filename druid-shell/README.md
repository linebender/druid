# druid-shell

`druid-shell` provides a common interface to the various elements of different platform application
frameworks. It is designed to be used by [Druid], a UI toolkit.

## Project status

`druid-shell` v0.8 was forked to form [Glazier], which is where all new development happens.
No further development is expected on `druid-shell`. We recommend everyone migrates to [Glazier].

## Design

The code in `druid-shell` can be divided into roughly two categories: the
platform agnostic code and types, which are exposed directly, and the
platform-specific implementations of these types, which live in per-backend
directories in `src/backend`. The backend-specific code for the current
backend is re-exported as `druid-shell::backend`.

`druid-shell` does not generally expose backend types directly. Instead, we
expose wrapper structs that define the common interface, and then call
corresponding methods on the concrete type for the current backend.

## Unsafe

Interacting with system APIs is inherently unsafe. One of the goals of
`druid-shell` is to handle all interaction with these APIs, exposing
a safe interface to `druid` and other possible consumers.

[Druid]: https://github.com/linebender/druid
[Glazier]: https://github.com/linebender/glazier
