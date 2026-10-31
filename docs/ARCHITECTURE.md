# Axis Architecture

This document records Axis's crate layout, the dependency graph, and — most
importantly — the reasoning behind the design decisions made so far. It is
updated as phases land.

## Crate layout

| Crate | Responsibility | Depends on |
|---|---|---|
| `axis-core` | Typed snowflake IDs, timestamps, gateway intents, permissions, the unified error system, credential redaction, optional logging setup | — |
| `axis-model` | Strongly typed Discord data models | `axis-core` |
| `axis` | Facade: re-exports the pieces applications touch, provides the `prelude` | `axis-core`, `axis-model`, `axis-rest` |
| `axis-rest` | Async REST client: auth, rate limits, typed request builders | `axis-core`, `axis-model` |
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
| `tokio` | Async runtime; pinned centrally in `[workspace.dependencies]` |

### Added in Phase 2 (REST)

| Dependency | Why |
|---|---|
| `reqwest` (json + rustls-tls, no default features) | Async HTTP; rustls avoids a system OpenSSL dependency |
| `mockito` (dev-dependency) | Local mock HTTP server for pipeline tests |

Deliberately absent: WebSocket libraries (`url`, `bytes`, `futures` remain
unexposed), database drivers — each arrives with the phase that needs it.

## Phase 2 design decisions (REST)

### 10. Rate limiting: learned buckets, keyed by (hash, major parameter)

`RateLimiter` follows the documented model rather than hardcoding limits:
the first request on a route shape always goes through (the bucket is
unknown); every response teaches the limiter the route's `X-RateLimit-Bucket`
hash and window state (`Remaining`, `Reset-After`). State is keyed by
`(bucket hash, major parameter)` so `/channels/123` and `/channels/456`
never share a limit. Waiting happens outside the lock (compute wait under
the mutex, sleep, re-check), so slow waiters cannot stall other tasks.

429 handling: the body's `retry_after` is authoritative, falling back to the
`Retry-After` header and a conservative 1s default (Cloudflare returns
HTML). Global 429s block all routes until expiry. Retries are bounded by
`max_rate_limit_retries` (default 3), after which a `RateLimitError` with
the parsed context surfaces — never an infinite retry loop.

### 11. Bounded, method-aware transient retries

Network errors and 5xx responses retry with exponential backoff (500ms
doubling, capped at 5s) — but **only for idempotent methods**
(`GET`/`PUT`/`DELETE`). A `POST` that hits a 500 fails immediately: retrying
message sends risks silent duplicates. `max_retries` (default 3) bounds the
loop. 401 stops immediately with `AuthError::Invalid`, per Discord's
guidance to stop using an invalid token.

### 12. Awaitable builders via `IntoFuture`

Every endpoint is a method on `Http` returning a typed builder; builders
implement `IntoFuture` (boxing one future per request — negligible next to
the HTTP round trip), so `http.create_message(id).content("hi").await`
matches the README's design target. Local validation happens inside
`execute` **before** any network call: content length (2000 chars),
non-empty message body, bulk-delete size (2–100), slowmode bounds
(≤ 21600s), timeout direction, and query-parameter exclusivity.

### 13. Null-clearing semantics

Edit endpoints that support explicit clearing (`EditMessage.content/embeds`,
`EditMember.nick/timeout_until`) model "set" vs "clear" with
`Option<Option<T>>`: outer `None` is skipped entirely, `Some(None)`
serializes as `null` (Discord's "remove this field"), `Some(Some(v))` sets.

### 14. Where permission errors live

HTTP 403 maps to `HttpError` (status 403 + Discord's code/message), because
Discord does not say *which* permission was missing. `PermissionError` with
its `missing: Permissions` payload is reserved for local checks that know
exactly what they require (the Phase 5 command framework's pre-checks).

## Roadmap

| Phase | Scope | Status |
|---|---|---|
| 1 | Workspace, core types, error system, first model, docs | ✅ done |
| 2 | REST: auth, HTTP client, rate limiting, message/channel/guild/member/role/reaction ops, mocked tests | ✅ done |
| 3 | Gateway: connect, identify, heartbeat, resume, reconnect, event dispatch | next |
| 4 | Models expansion + event system + handlers | ⏳ |
| 5 | Interactions and slash-command framework | ⏳ |
| 6 | Examples, integration/long-run testing, docs polish | ⏳ |

## Testing strategy

- Unit tests live next to the code (every module in `axis-core`,
  `axis-model`, and `axis-rest` has a `#[cfg(test)] mod tests`).
- Documented values from the Discord spec are pinned as test constants
  (intent bits, permission bits, close codes, snowflake → timestamp
  examples, message/channel/role payloads).
- The REST pipeline is tested against a local mock HTTP server (`mockito`):
  request shapes (method, path, headers, bodies, query strings), rate-limit
  behavior (bucket waiting, bounded 429 retries, global limits), retry
  rules (5xx on idempotent methods only), and error mapping (401/403/404/500).
- Cross-crate behavior is tested through the facade in `tests/`
  (`axis-integration-tests`), which is how applications consume the library.
- Phases 3+ add mocked gateway frames (heartbeat acks, close codes, resume
  flows) — live Discord connections are never the only test path.
