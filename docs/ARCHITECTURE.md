# Axis Architecture

This document records Axis's crate layout, the dependency graph, and — most
importantly — the reasoning behind the design decisions made so far. It is
updated as phases land.

## Crate layout

| Crate | Responsibility | Depends on |
|---|---|---|
| `axis-core` | Typed snowflake IDs, timestamps, gateway intents, permissions, the unified error system, credential redaction, optional logging setup | — |
| `axis-model` | Strongly typed Discord data models | `axis-core` |
| `axis` | Facade: the `Client`, re-exports, and the `prelude` | `axis-core`, `axis-model`, `axis-rest`, `axis-gateway`, `axis-events`, `axis-commands` |
| `axis-rest` | Async REST client: auth, rate limits, typed request builders | `axis-core`, `axis-model` |
| `axis-gateway` | Gateway WebSocket client: identify, heartbeat, resume, reconnect | `axis-core` |
| `axis-events` | Typed event parsing and asynchronous handler dispatch | `axis-core`, `axis-model`, `axis-gateway` |
| `axis-commands` | Slash-command framework on top of interactions | `axis-core`, `axis-model`, `axis-rest` |
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

### Added in Phase 3 (Gateway)

| Dependency | Why |
|---|---|
| `tokio-tungstenite` (rustls-tls-webpki-roots) | Async WebSocket transport; rustls avoids a system OpenSSL dependency |
| `futures-util` | Stream/Sink combinators for the split WebSocket |

Deliberately absent: `url`, `bytes`, database drivers — each arrives with
the phase that needs it.

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
| 3 | Gateway: connect, identify, heartbeat, resume, reconnect, event dispatch | ✅ done |
| 4 | Models expansion + event system + handlers | ✅ done |
| 5 | Interactions and slash-command framework | ✅ done |
| 6 | Examples, integration/long-run testing, docs polish | next |

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

## Phase 3 design decisions (Gateway)

### 15. One shard, one supervision task

`Shard` owns a single gateway connection driven by one task that `select!`s
over socket reads, heartbeat deadlines, and the shutdown signal. The
WebSocket is split into disjoint sink/source halves at the runner level so
heartbeats can be written while a read is being awaited (two `&mut` borrows
of one struct would not compose inside `select!`). The event channel is
unbounded: a stalled consumer must never block the heartbeat loop, or the
shard would disconnect itself.

### 16. Resume-vs-identify decided by the documented triggers

`Session` holds `session_id`, last sequence, and `resume_gateway_url`.
Reconnection policy, all sourced from the docs:

- op 7 (Reconnect), op 9 with `d: true`, transport loss, zombie heartbeats,
  and resumable close codes → new connection to `resume_gateway_url` +
  **Resume** with the stored `session_id`/`seq`.
- op 9 with `d: false`, close codes 1000/1001 (which invalidate sessions),
  and 4007/4009 (docs: "reconnect and start a new session") → **fresh
  Identify** at the configured gateway URL; `Session::reset()` clears the
  resume URL so the stale one is never reused.
- 4004 and 4010-4014 → stream ends with a typed `GatewayError::Closed`;
  retrying cannot fix a bad token or intents configuration.

Zombie detection follows the docs exactly: if no Heartbeat ACK arrives
between two heartbeat attempts, terminate with a close code other than
1000/1001 and resume. The first heartbeat after Hello is sent after
`heartbeat_interval * jitter` (also per the docs), and server-requested
heartbeats (op 1) are answered immediately.

### 17. Raw dispatches now, typed events next

`ShardEvent::Dispatch` forwards `{kind, data}` untouched: sequence and
session bookkeeping stay internal (they exist for resume), and typed event
parsing is Phase 4's `axis-events` concern. Lifecycle events (`Connected`,
`Resumed`, `Reconnecting`) are surfaced for observability and because READY
alone cannot tell you a resume happened.

### 18. Sharding-ready by construction

One `Shard` is one connection with a shard index/count pair in Identify.
Multi-shard management (plus session-start-limit accounting from
`GET /gateway/bot`) is deferred to a coordinating manager once the event
system exists — the per-connection design needed for it is already in
place.

### Testing the gateway without Discord

The pipeline tests run a real local WebSocket server (the same
`tokio-tungstenite` crate, server side) with scripted per-connection
handlers: identify shape (token prefix stripping, intents, properties),
heartbeat/ack flow with sequence numbers, op 7 → Resume, op 9 both ways,
close codes 4004 (fatal, no reconnect) and 4007 (fresh identify), zombie
teardown, abrupt transport drops, graceful shutdown (close frame 1000),
and the reconnect-attempts cap. `resume_gateway_url` in the fixture points
back at the test server, so the full reconnect path — including URL
switching — is exercised over real sockets.

## Phase 4 design decisions (Models & events)

### 19. Unknown events are data, not errors

Event kinds Axis does not model yet (Discord ships new ones regularly)
surface as `Event::Unknown { kind, data }` — a running bot must survive
API growth. Within known kinds the same philosophy continues: unknown JSON
fields are ignored. Parse failures for *known* kinds do return errors from
`Event::parse`, but the dispatcher's run loop logs and skips them: one
malformed dispatch must not end a long-running bot. Only gateway stream
errors are fatal, because the shard itself has given up.

### 20. Handler semantics: ordered, per-event, survivable

Handlers are async, stored per event kind, and awaited in registration
order (predictable ordering; no cross-event concurrency hazards). The
catch-all (`on_any`) runs before typed handlers and receives a cloned
event (owned `Event`, since `&Event` borrows cannot support `'static`
futures). A handler error stops later handlers *for that event* and is
returned from `dispatch` — but the `run` loop logs it and continues, so
one failing handler never kills the bot.

### 21. Event payloads are flat, per the wire

Payloads that Discord flattens into their parent object
(`GUILD_MEMBER_ADD`/`UPDATE` put `guild_id` beside member fields) are
modeled with `#[serde(flatten)]`; `GUILD_CREATE` flattens the standard
guild object plus its connection extras. `MESSAGE_UPDATE` gets a dedicated
`PartialMessage` (the docs make most fields partial there) instead of
pretending it is a full `Message`.

### 22. Interactions: typed envelope, raw data (for now)

`Interaction` is fully typed (id, type, guild/channel, member/user,
token, permissions); the `data` field stays raw because its shape differs
per interaction type. `Interaction::command_data()` decodes the
application-command shape (including `ResolvedData`), and Phase 5 builds
the typed per-kind enum plus the command framework on top.

## Phase 5 design decisions (Commands & Client)

### 23. The Client composes; it does not absorb

`Client` is thin composition in the facade crate: an `Arc<Http>`, a
`Shard`, and a `Dispatcher` (optionally bridged to a `CommandRegistry`).
Subsystem APIs remain directly reachable (`Client::http()`, `Shard`
configuration, `Dispatcher` semantics), and `run_with` accepts any event
stream, which keeps embedding and testing possible without a live gateway.
`build()` fetches `GET /gateway` for the recommended URL — overridable for
tests. Event handlers receive a `Context` that carries the REST client;
the command framework deliberately does not depend on it: `Invocation`
carries its own `Arc<Http>`, so `axis-commands` sits below the facade in
the dependency graph.

### 24. Registration by bulk overwrite

The registry serializes to `ApplicationCommandFields` and syncs through
`PUT /applications/{id}/commands` (or the guild variant) — the documented
way to make the API match the declaration set. Declaration validation
(name rules, 1-100 descriptions, required-before-optional ordering, no
choices+autocomplete mixing, handler presence) runs client-side first:
invalid declarations fail locally instead of spending a Discord round
trip and an invalid-request strike.

### 25. Interaction tokens are credentials in URLs

The callback and followup endpoints embed the interaction token in the
path (as the API requires). Routes therefore carry a sanitized `display`
form (`/interactions/900/[token]/callback`) and every error and log line
uses it — the token cannot leak through `HttpError`. The general rule
established in Phase 2 (bounded retries) interacts here with the 3-second
initial-response deadline: interaction responses are POSTs, which the
pipeline never retries on 5xx, so no double-response risk exists.

### 26. Commands are a tree walked at dispatch time

The registry stores a two-level tree (leaf commands, subcommands, one
group level) matching Discord's shape. `resolve` walks the incoming
options to the leaf handler and records the subcommand path; the
invocation's argument access reads options at that path's scope. Options
stay handler-side typed via `FromCommandValue` (String, i64, f64, bool,
and snowflake IDs) rather than a macro-driven DSL — simple, inspectable,
and sufficient for the documented option kinds. Autocomplete providers
receive the same `Invocation` and answer through the type-8 callback;
requests without a focused option or provider are answered with empty
choices rather than left hanging (the token dies if nothing responds).
