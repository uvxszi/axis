# Examples

Executable examples live here, one directory per example, wired as workspace
members or as `[[example]]` targets of the `axis` facade crate.

## Status

Examples are delivered together with the subsystems they demonstrate, so
that every example is a **working, compilable program** — not a sketch:

| Example | Demonstrates | Lands in |
|---|---|---|
| `basic_bot` | Client setup, token from environment, intents, first handler | Phase 6 (after REST + gateway exist) |
| `event_handler` | Registering handlers for the main lifecycle events | Phase 4–6 |
| `slash_commands` | Command registration, options, interactions, ephemeral/deferred responses | Phase 5–6 |

Until those phases are implemented there is deliberately nothing runnable in
this directory: the `Client` API shown in the README is the design target
for Phases 2–5. See `docs/ARCHITECTURE.md` for the roadmap.
