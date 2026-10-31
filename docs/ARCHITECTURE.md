# Axis Architecture

This document records Axis's crate layout, the dependency graph, and — most
importantly — the reasoning behind the design decisions made so far. It is
updated as phases land.

## Crate layout

| Crate | Responsibility | Depends on |
|---|---|---|
| `axis-core` | Typed snowflake IDs, timestamps, gateway intents, permissions, the unified error system, credential redaction, optional logging setup | — |
| `axis-model` | Strongly typed Discord data models | `axis-core` |
| `axis` | Facade: re-exports the pieces applications touch, provides the `prelude` | `axis-core`, `axis-model` |
| `axis-rest` | Async REST client: auth, rate limits, typed request builders *(Phase 2)* | `axis-core`, `axis-model` |
| `axis-gateway` | Gateway WebSocket client: identify, heartbeat, resume, sharding *(Phase 3)* | `axis-core`, `axis-model`, `axis-rest` (API version / URL negotiation) |
| `axis-events` | Event parsing and asynchronous handler dispatch *(Phase 4)* | `axis-core`, `axis-model`, `axis-gateway` |
| `axis-commands` | Slash-command framework on top of interactions *(Phase 5)* | `axis-core`, `axis-model`, `axis-rest`, `axis-events` |
| `tests` (`axis-integration-tests`) | Cross-crate integration tests through the public facade API | `axis` |

```
                    ┌──────────────┐
                    │    axis      │  (facade + prelude)
                    └──────┬───────┘
              ┌────────────┼───────────────┐
        ┌─────▼─────┐ ┌────▼─────┐   ┌─────▼────────┐
        │axis-model │ │axis-core │   │ axis-rest    │   axis-gateway
        └─────┬─────┘ └────▲─────┘   └──────┬───────┘   axis-events
              └────────────┘                ▼           axis-commands
                                     (Phase 2+) — later crates also sit on
                                      axis-core + axis-model
```

Guiding rule: **the split exists to keep dependency weight proportional to
need**. `axis-core` compiles with no async runtime, no HTTP client, and no
WebSocket library, so any consumer can reuse just the types without pulling
in Tokio or Reqwest. Heavier dependencies land only in the crates that
actually use them. Crates are *not* split further without a concrete reason
(e.g. a feature gate boundary or an optional dependency).

## Design decisions

### 1. Generic `Id<T>` with marker types, not per-entity newtypes

Snowflake IDs are one struct (`Id<T>`) parameterized by zero-sized marker
types (`marker::UserMarker`, ...), with short aliases (`UserId`,
`GuildId`, ...). Rationale:

- One serde/`Display`/`Hash`/`Ord` implementation instead of a dozen
  macro-generated near-duplicates.
- Full compile-time safety: `UserId` and `ChannelId` are distinct types, so
  mixing them up does not compile.
- Markers are suffixed `...Marker` so they never collide with data models of
  the same name (`User`).
- Serde accepts both the number and the decimal-string form Discord uses in
  different endpoints, and rejects negative values.

### 2. Error system: one top-level enum, self-describing sub-errors

`AxisError` groups failures by subsystem (auth, connection, gateway, HTTP,
invalid data, internal, permission, rate limit, serialization). Decisions:

- Each sub-error is a real type with structured, pub fields (status + route
  + Discord error code for HTTP; retry duration + bucket for rate limits;
  close code + reason for gateway closes) — errors are data, not strings.
- `Display` for `AxisError` forwards (`#[error("{0}")]`) to sub-errors that
  carry self-describing messages. `#[error(transparent)]` was rejected: it
  also elides the sub-error from `source()`, destroying the middle of the
  chain (gateway error → transport error).
- `is_transient()` gives callers a conservative default for retry decisions.
- `GatewayCloseCode` encodes Discord's documented close-code table with
  `is_reconnectable()` / `should_resume()`: codes 4004, 4010–4014 stop
  reconnection; 4007/4009 allow reconnect but explicitly require a fresh
  session instead of a resume. The gateway phase builds on this instead of
  re-deriving it.
- HTTP statuses are stored as `u16` so `axis-core` does not depend on
  `reqwest`/`http`; the REST phase maps rich responses into these types.

### 3. Bitsets via `bitflags`, serialized the way Discord does it

- `GatewayIntents`: strict deserialization (unknown bits are rejected) —
  sending an unknown intent makes Discord close the connection with 4013.
- `Permissions` / `UserFlags`: lenient deserialization — unknown bits are
  kept (permissions) or dropped (flags), because Discord adds bits over
  time and an unknown bit must not break a running bot.
- Discord serializes permission bitsets as decimal **strings** in role and
  overwrite objects; `Permissions` therefore serializes to a string and
  accepts string or number input. Intents are JSON numbers in the Identify
  payload, so they serialize as numbers.

### 4. Spec fidelity over memory

Intent bit values, permission bits, user flags, field names/optionality,
close codes, and the snowflake timestamp layout were taken from the official
Discord documentation and pinned by unit tests, so drift is caught in CI.

### 5. Robustness against API drift

- Unknown JSON fields are ignored (never `deny_unknown_fields`).
- Optional Discord fields are `Option<T>`; presence semantics mirror the
  documented optional/nullable markers.
- Enum values arriving as unknown numbers surface a deserialization error
  rather than being silently misinterpreted (an explicit decision; may gain
  catch-all variants later if field experience demands it).

### 6. Credentials never leak

`Secret<T>` wraps credentials with a `Debug` impl that prints
`Secret([REDACTED])`, no `Display` impl, and no serde impls — three separate
accidental-leak channels closed. Access is explicit (`expose`/`into_inner`).
`Token` (=`Secret<String>`) additionally offers `from_env`, trimming
whitespace and treating empty values as missing. The REST phase builds
authentication on this type.

### 7. `time` over `chrono`

`Timestamp` wraps `time::OffsetDateTime`. `time` is smaller, MSRV-friendly,
and its `Rfc3339` well-known format matches Discord's timestamp format
exactly. `chrono` would add compile time for capabilities Axis does not use.

### 8. Logging: `tracing` facade, never implicit global state

Axis emits through `tracing` and installs no subscriber on its own. The
`log` module offers opt-in helpers (`init`, `init_with_filter`) for
applications that want a default subscriber; library consumers and embedders
can ignore them entirely.

### 9. No mandatory storage layer

Per requirements, no database dependency exists anywhere in the workspace.
Persistence is an application concern (an optional integration crate could
come later, but the core stays storage-free).

## Dependency choices (Phase 1)

| Dependency | Why |
|---|---|
| `serde` / `serde_json` | Serialization backbone of everything Discord |
| `thiserror` | Idiomatic, low-machinery error derives |
| `bitflags` | Battle-tested bitset codegen for intents/permissions/flags |
| `time` | RFC 3339 timestamps |
| `tracing` / `tracing-subscriber` | Structured logging; subscriber only for the opt-in helper |
| `tokio` | Declared in `[workspace.dependencies]` for Phase 2+ so versions are pinned centrally |

Deliberately absent so far: `reqwest`, WebSocket libraries, `url`, `bytes`,
database drivers — each arrives with the phase that needs it.

## Roadmap

| Phase | Scope | Status |
|---|---|---|
| 1 | Workspace, core types, error system, first model, docs | ✅ done |
| 2 | REST: auth, HTTP client, rate limiting, message ops, mocked tests | next |
| 3 | Gateway: connect, identify, heartbeat, resume, reconnect, event dispatch | ⏳ |
| 4 | Models expansion + event system + handlers | ⏳ |
| 5 | Interactions and slash-command framework | ⏳ |
| 6 | Examples, integration/long-run testing, docs polish | ⏳ |

## Testing strategy

- Unit tests live next to the code (every module in `axis-core` and
  `axis-model` has a `#[cfg(test)] mod tests`).
- Documented values from the Discord spec are pinned as test constants
  (intent bits, permission bits, close codes, snowflake → timestamp
  examples).
- Cross-crate behavior is tested through the facade in `tests/`
  (`axis-integration-tests`), which is how applications consume the library.
- Phases 2+ add mocked HTTP responses (rate limits, error bodies) and mocked
  gateway frames (heartbeat acks, close codes, resume flows) — live Discord
  connections are never the only test path.
