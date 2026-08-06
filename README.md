# BELLA — Project Handoff / Continuation Instructions

**Read this file first, before touching any code.** It exists so a new AI
session (or a new engineer) can pick up exactly where the last session
left off, without re-litigating decisions that were already made and
tested. If anything here conflicts with what you find in the repo, the
repo (working code + passing tests) is the source of truth — update this
file to match reality, don't trust stale prose over code.

---

## 1. What this project is

Bella is a JARVIS/Ultron-inspired **personal AI OS**: voice, vision,
long-term memory, device control, automation, and a plugin system,
running as a local-first Rust daemon on Linux desktop. It was originally
named "Ultron" and renamed to "Bella" — if you see `ultron` anywhere
(crate names, comments, git history), that's a leftover from before the
rename and should be fixed, not treated as intentional.

**Owner context:** built by a CS student focused on language design,
systems programming, and Linux desktop (Rust/C++/Go/QML), with an
existing Hyprland/QuickShell dotfiles ecosystem — Bella should eventually
integrate with that, not compete with it.

## 2. The ground rules for how this project is built

These were established explicitly and must not be silently dropped by a
new session:

1. **Phase discipline.** Work proceeds in named phases (Requirements →
   Architecture → Stack → Repo Structure → Core AI → Voice → Vision →
   Memory → Device Control → ... see the full 20-phase list in
   `docs/adr/0001-original-brief.md`). Do not skip ahead or merge phases.
   Do not build Vision before Voice's real implementation exists, etc.
2. **No fake code.** No placeholder implementations, no `TODO: implement
   later`, no stub functions that return hardcoded values pretending to
   be real. If something is genuinely deferred (e.g. semantic vector
   search), say so explicitly in a doc comment — don't fake it silently.
3. **Everything must actually run.** Every deliverable in this project so
   far has been built, `cargo test`-ed, and `cargo run` executed live to
   observe real output — not just "this should compile." Continue that
   standard: don't hand back code you haven't actually built and run in
   this sandbox.
4. **Interface-first, dependency graph enforced.** Subsystems talk to
   each other only through `bella-common`'s `MessageBus`/`Envelope`/
   `Payload` contract, never by holding direct references to each
   other's internals. The dependency direction is (see
   `docs/architecture.md` for the diagram):
   `Permissions ← Memory ← Context ← Reasoning ← Action Router ← Device/Plugins`.
   No crate below Action Router may depend on Context or Reasoning.
5. **Rust for the core daemon.** This was explicitly revisited (a C
   detour was considered and reverted) — Rust stays, for memory safety in
   a privileged, mic/filesystem/shell-access process. Don't re-relitigate
   this unless the user raises it again.
6. **Cloud API for reasoning, v1.** Local LLM inference is explicitly
   deferred, not forgotten — the `ReasoningEngine` trait (next
   deliverable, see §5) must be designed so a local implementation is a
   second `impl`, not a rewrite.
7. **Security is not bolted on.** Every privileged action must go through
   `bella-permissions::PermissionSystem::check()`, which audits every
   decision. No subsystem gets ambient authority.

## 3. Current repo state (as of this handoff)

```
bella/
├── Cargo.toml                       # workspace root, pins uuid=1.8.0, tempfile=3.10.1
│                                     # (pinned because this sandbox has an old rustc/cargo
│                                     #  1.75 via apt — newer transitive deps require
│                                     #  edition2024. If you have a modern toolchain,
│                                     #  these pins can likely be relaxed — try it and see.)
├── crates/
│   ├── bella-common/     # DONE, tested (4 tests)
│   │   src/
│   │     error.rs        # BellaError / BellaResult — the crate-wide error type
│   │     subsystem.rs    # SubsystemId enum — the addressing scheme
│   │     message.rs      # Envelope + Payload — the message contract between subsystems
│   │     bus.rs          # MessageBus — tokio mpsc-based, bounded (256/inbox), backpressure-safe
│   ├── bella-permissions/ # DONE, tested (7 tests)
│   │   src/
│   │     capability.rs   # Capability enum (Microphone, ShellExec, FilesystemRead{prefix}, ...)
│   │     grant.rs        # Grant, Grantee, AuditEntry/AuditDecision
│   │     system.rs       # PermissionSystem: grant/revoke/check, append-only audit log
│   ├── bella-memory/     # DONE, tested (6 tests)
│   │   src/
│   │     types.rs        # Episode, NewEpisode
│   │     store.rs        # MemoryStore — sync rusqlite layer, all raw SQL lives ONLY here
│   │     engine.rs        # MemoryEngine — async wrapper via spawn_blocking, public API
│   ├── bella-core/       # daemon: Subsystem trait, Supervisor, demo subsystems, main.rs
│   │   src/
│   │     subsystem_trait.rs  # Subsystem trait every subsystem implements
│   │     supervisor.rs       # Supervisor: spawns + restarts subsystems on panic (max 5 restarts, 500ms backoff)
│   │     demo_subsystems.rs  # DemoVoiceSubsystem + DemoContextSubsystem — REAL reference
│   │                          # implementations (not mocks) proving Voice -> permission
│   │                          # check -> Context -> Memory Engine write+recall works
│   │                          # end-to-end. These are NOT the final Voice/Context
│   │                          # subsystems (no whisper.cpp/piper wired in yet) — they're
│   │                          # the honest scaffolding Phase 6/Context-Builder-proper
│   │                          # will replace.
│   │     main.rs             # entrypoint: wires bus+permissions+memory, runs a 2-utterance
│   │                          # demo proving cross-interaction memory recall, then waits
│   │                          # for Ctrl+C
├── docs/
│   ├── architecture.md   # Phase 2 diagrams/decisions (create this if missing — see §6)
│   └── adr/               # Architecture Decision Records, one per major call
│       └── 0001-original-brief.md  # the original 20-phase brief, verbatim, for reference
```

**Verification commands** (all currently pass in this sandbox):
```bash
cargo build          # clean, zero warnings
cargo test            # 17/17 passing (4 common + 6 memory + 7 permissions)
cargo run --bin bellad  # runs live, logs Voice->Permission->Context->Memory pipeline,
                         # writes bella_memory.db in the working directory, waits on Ctrl+C
```
Note: `cargo clippy` could not be run in this sandbox (no `rustup`, no
`cargo-clippy` package available via apt). Run it on a real dev machine
before merging anything — it was never actually verified, don't claim it
was.

## 4. What is explicitly NOT done yet (don't assume otherwise)

- **Reasoning Engine** — no `ReasoningEngine` trait exists yet, no Claude
  API integration. This is the next deliverable (see §5).
- **Action Router** — doesn't exist. Nothing currently validates/dispatches
  proposed actions; there are no actions being proposed yet since there's
  no Reasoning Engine.
- **Real Voice subsystem** — `DemoVoiceSubsystem` is a stand-in. No
  whisper.cpp, no wake word, no TTS. It correctly demonstrates the
  message-passing and permission-check pattern the real one will follow.
- **Real Context Builder** — `DemoContextSubsystem` writes to memory and
  recalls recent episodes, which is real, but it doesn't yet *assemble* a
  bounded prompt context (token budgeting, relevance filtering) — it just
  logs what it recalled.
- **Vector/semantic search** — `MemoryStore::search_content` is a `LIKE`
  substring search, explicitly a placeholder. `sqlite-vec` was the Phase 3
  choice but isn't integrated.
- **Vision, Device Control, Automation, Plugin SDK, everything from Phase
  6 onward** — not started.

## 5. Immediate next task: the Reasoning Engine

This is what the next session should build first, following the existing
patterns:

- New crate `bella-reasoning`, depending on `bella-common` (and
  `bella-memory` if it needs `Episode` types directly).
- Define a `ReasoningEngine` trait (likely `#[async_trait]`, matching the
  style of `Subsystem` in `bella-core/src/subsystem_trait.rs`):
  roughly `async fn reason(&self, context: AssembledContext) -> BellaResult<ProposedAction>`.
  Look at `Payload::AssembledContext` and `Payload::ProposedAction` in
  `bella-common/src/message.rs` — the shape of those payloads should
  inform the trait's associated types, or you may find those payloads
  need to grow richer fields now that there's a real consumer.
- Implement `CloudReasoner` — calls the Claude API over HTTPS
  (`reqwest` is a reasonable new dependency; nothing in the workspace
  pulls it in yet). Needs an API key sourced via `libsecret`/OS keyring
  eventually (Phase 1 secrets requirement) — for a first pass, reading
  from an environment variable is an acceptable, explicitly-labeled
  stopgap, but say so in a comment, don't pretend it's the final answer.
- Wire it into `bella-core`: extend or replace `DemoContextSubsystem` so
  that after recording+recalling memory, it actually calls the
  `ReasoningEngine` and forwards the resulting `ProposedAction` onward
  (there's no Action Router yet, so for now logging the proposed action
  is an honest stopping point — don't fabricate an Action Router just to
  have somewhere to send it).
- Write tests. The existing crates all have real unit tests including at
  least one deliberately-adversarial case (backpressure, expired grants,
  ordering ties) — match that bar. For `CloudReasoner` specifically,
  you'll likely want the trait design to allow a mock implementation in
  tests rather than hitting the real API from `cargo test`.
- Build, test, and **actually run `cargo run --bin bellad`** to observe
  real output before calling it done, same as every prior deliverable.

## 6. If `docs/architecture.md` or `docs/adr/` don't exist yet

The Phase 2 architecture diagram and the Phase 1 requirements/Phase 3
stack table/Phase 4 repo structure were produced in chat but may not have
been written to files yet. If they're missing from `docs/`, reconstruct
them from this handoff's summary and the chat history if available, and
write them down — they should not keep living only as chat scrollback.
