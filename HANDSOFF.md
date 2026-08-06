# BELLA — Engineering Handoff & Continuation Guide

> **Project status, architectural constraints, development standards, and next implementation milestone.**

This document is the primary engineering handoff for BELLA. It records the current implementation state, architectural decisions, development constraints, completed milestones, known gaps, and the next required development work.

It is intended for any developer or contributor continuing the project.

Before modifying the architecture or implementing a new subsystem, review this document and the referenced architecture decision records.

The repository remains the authoritative source of truth. If documentation conflicts with working, tested code, the implementation takes precedence and the documentation should be updated accordingly.

---

# 1. Project Overview

BELLA is a JARVIS/Ultron-inspired **personal AI operating system** designed around:

* Voice interaction
* Vision
* Long-term memory
* Device control
* Automation
* Plugin extensibility
* Local-first execution

The core system runs as a Rust daemon on Linux.

The project was originally named **Ultron** and was later renamed to **BELLA**.

Any remaining `ultron` identifiers in crate names, comments, documentation, configuration, or other project files are legacy artifacts from the previous project name and should be migrated to `bella` when encountered.

They should not be treated as intentional naming.

## Project Direction

BELLA is being developed alongside an existing Linux desktop ecosystem centered around:

* Hyprland
* QuickShell
* Rust
* C++
* Go
* QML

BELLA should eventually integrate with this ecosystem rather than attempting to replace or duplicate it.

---

# 2. Engineering Principles

The following rules are established project requirements.

They should remain consistent throughout future development unless an explicit architectural decision changes them.

---

## 2.1 Phase-Driven Development

Development follows a defined sequence of phases:

**Requirements → Architecture → Stack → Repository Structure → Core AI → Voice → Vision → Memory → Device Control → ...**

The complete 20-phase development plan is documented in:

`docs/adr/0001-original-brief.md`

Development must respect these phase boundaries.

Subsystems should not be implemented before the dependencies required by their phase are complete.

For example:

**Vision implementation depends on the production Voice implementation being completed first.**

Phases should not be silently skipped, merged, or reordered.

Any intentional change to the phase plan should be documented as an architectural decision.

---

## 2.2 Production-Real Implementations

BELLA does not use fake implementations to represent unfinished functionality.

The following patterns are prohibited as substitutes for actual implementation:

* Placeholder functions returning hardcoded results
* Fake subsystem behavior presented as production behavior
* `TODO: implement later` implementations masquerading as completed functionality
* Silent stubs
* Hardcoded responses used to imply a subsystem exists

When functionality is intentionally deferred, the limitation should be stated explicitly in code comments or project documentation.

For example, semantic vector search is intentionally deferred and should remain documented as such until implemented.

---

## 2.3 Runtime Verification

Compilation alone is not sufficient for a BELLA milestone to be considered complete.

Every deliverable is expected to be:

1. Implemented
2. Built
3. Tested
4. Executed
5. Observed under real runtime conditions

The established verification standard is:

```bash
cargo build
cargo test
cargo run --bin bellad
```

Code should not be considered complete until the relevant implementation has actually been executed.

This requirement exists because successful compilation does not guarantee correct subsystem interaction or runtime behavior.

---

## 2.4 Interface-First Architecture

BELLA uses strict subsystem boundaries.

Subsystems communicate through the contracts defined in `bella-common`.

The primary communication primitives are:

```text
MessageBus
Envelope
Payload
```

Subsystems should not maintain direct references to the internal implementation of unrelated subsystems.

The dependency direction is:

```text
Permissions
    ↑
Memory
    ↑
Context
    ↑
Reasoning
    ↑
Action Router
    ↑
Device / Plugins
```

The authoritative architecture diagram belongs in:

```text
docs/architecture.md
```

A crate below the Action Router layer must not introduce dependencies on Context or Reasoning internals.

Communication across these boundaries should occur through the established message contract.

---

## 2.5 Rust Core

The BELLA daemon remains implemented in **Rust**.

A C-based core was previously considered and intentionally rejected.

Rust was retained because BELLA is expected to operate with privileged access to resources including:

* Microphone
* Filesystem
* Shell execution
* Device interfaces
* Automation
* Plugins

Memory safety is therefore an architectural requirement rather than merely a language preference.

Changing the core implementation language would require a new architectural decision and should not happen as an incidental refactor.

---

## 2.6 Reasoning Backend Strategy

Version 1 uses a **cloud API for reasoning**.

Local LLM inference is intentionally deferred.

This does not mean local inference has been abandoned.

The Reasoning Engine architecture must support multiple implementations behind the same abstraction.

Conceptually:

```text
ReasoningEngine
├── CloudReasoner
└── LocalReasoner        # future implementation
```

Adding local inference later should require another implementation of the reasoning interface rather than rewriting the reasoning architecture.

---

## 2.7 Security Model

Security is part of the architecture rather than a feature added after subsystem implementation.

Every privileged action must pass through:

```rust
bella_permissions::PermissionSystem::check()
```

The permission system records every authorization decision in its audit log.

No subsystem should receive unrestricted ambient authority over privileged operations.

This applies to capabilities such as:

* Microphone access
* Shell execution
* Filesystem access
* Device control
* Future automation capabilities

Permission checks should remain explicit and auditable.

---

# 3. Current Repository State

The current repository structure is:

```text
bella/
├── Cargo.toml
│
├── crates/
│   │
│   ├── bella-common/
│   │   └── src/
│   │       ├── error.rs
│   │       ├── subsystem.rs
│   │       ├── message.rs
│   │       └── bus.rs
│   │
│   ├── bella-permissions/
│   │   └── src/
│   │       ├── capability.rs
│   │       ├── grant.rs
│   │       └── system.rs
│   │
│   ├── bella-memory/
│   │   └── src/
│   │       ├── types.rs
│   │       ├── store.rs
│   │       └── engine.rs
│   │
│   └── bella-core/
│       └── src/
│           ├── subsystem_trait.rs
│           ├── supervisor.rs
│           ├── demo_subsystems.rs
│           └── main.rs
│
└── docs/
    ├── architecture.md
    └── adr/
        └── 0001-original-brief.md
```

---

# 4. Implemented Components

## 4.1 `bella-common`

**Status: COMPLETE**

Current test count:

```text
4 tests
```

### `error.rs`

Defines the project-wide error model:

```text
BellaError
BellaResult
```

These types form the common error contract used across crates.

### `subsystem.rs`

Defines:

```text
SubsystemId
```

`SubsystemId` provides the addressing scheme used by BELLA's subsystem communication architecture.

### `message.rs`

Defines the communication contract:

```text
Envelope
Payload
```

These structures define how data moves between BELLA subsystems.

### `bus.rs`

Implements:

```text
MessageBus
```

The current bus uses Tokio MPSC channels.

Current characteristics:

```text
Inbox capacity: 256 messages
Bounded channels
Backpressure-safe
Async message delivery
```

---

# 5. Permission System

## `bella-permissions`

**Status: COMPLETE**

Current test count:

```text
7 tests
```

### `capability.rs`

Defines privileged capabilities such as:

```text
Microphone
ShellExec
FilesystemRead { prefix }
...
```

### `grant.rs`

Defines:

```text
Grant
Grantee
AuditEntry
AuditDecision
```

### `system.rs`

Implements:

```text
PermissionSystem
```

Current functionality includes:

```text
grant
revoke
check
append-only audit logging
```

Every permission decision is recorded.

This system forms the security boundary for privileged BELLA functionality.

---

# 6. Memory System

## `bella-memory`

**Status: COMPLETE**

Current test count:

```text
6 tests
```

### `types.rs`

Defines:

```text
Episode
NewEpisode
```

### `store.rs`

Implements the synchronous SQLite storage layer:

```text
MemoryStore
```

All raw SQL is intentionally isolated inside this layer.

Other crates should not introduce raw SQL for BELLA memory operations.

### `engine.rs`

Implements:

```text
MemoryEngine
```

`MemoryEngine` provides the asynchronous public memory API.

Blocking SQLite operations are executed through:

```rust
spawn_blocking
```

This prevents synchronous database operations from blocking Tokio's async runtime.

---

# 7. Core Daemon

## `bella-core`

The core daemon currently contains the subsystem runtime and integration scaffolding.

### `subsystem_trait.rs`

Defines the:

```text
Subsystem
```

trait.

Every BELLA subsystem follows this interface.

### `supervisor.rs`

Implements the subsystem:

```text
Supervisor
```

The supervisor starts and monitors subsystem tasks.

Current restart policy:

```text
Maximum restarts: 5
Restart backoff: 500 ms
```

Subsystems that panic can therefore be restarted within the defined recovery limit.

### `demo_subsystems.rs`

Contains:

```text
DemoVoiceSubsystem
DemoContextSubsystem
```

These are **real reference implementations**, but they are not the final production Voice and Context subsystems.

Their purpose is to prove that the architecture works end-to-end.

The currently demonstrated pipeline is:

```text
Voice
  ↓
Permission Check
  ↓
Context
  ↓
Memory Engine
  ↓
Write + Recall
```

These implementations should not be misrepresented as production Voice or production Context.

The production Voice implementation will eventually replace `DemoVoiceSubsystem`.

The production Context Builder will eventually replace the limited context behavior currently demonstrated by `DemoContextSubsystem`.

### `main.rs`

The current daemon entry point:

1. Initializes the message bus.
2. Initializes permissions.
3. Initializes memory.
4. Runs a two-utterance demonstration.
5. Demonstrates cross-interaction memory recall.
6. Waits for `Ctrl+C`.

The demo proves that BELLA can currently move information across subsystem boundaries and persist/retrieve memory.

---

# 8. Workspace Dependency Notes

The workspace currently pins:

```text
uuid = 1.8.0
tempfile = 3.10.1
```

These versions were pinned because the development sandbox used:

```text
rustc/cargo 1.75
```

Some newer transitive dependencies require Edition 2024 support and therefore cannot compile under that toolchain.

On a modern Rust toolchain, these pins may no longer be necessary.

They may be relaxed after verifying that:

```bash
cargo build
cargo test
```

continue to pass.

Dependency versions should not be upgraded blindly without verification.

---

# 9. Current Verification State

The current known verification commands are:

```bash
cargo build
cargo test
cargo run --bin bellad
```

Current known result:

```text
cargo build          → clean build, zero warnings
cargo test           → 17 / 17 tests passing
cargo run --bin bellad
                     → Voice → Permission → Context → Memory pipeline executes
```

Test distribution:

```text
bella-common       4
bella-memory       6
bella-permissions  7
--------------------
Total              17
```

Running the daemon creates:

```text
bella_memory.db
```

in the working directory.

The daemon then waits for `Ctrl+C`.

---

# 10. Clippy Verification Status

`cargo clippy` has **not been verified** in the original development environment.

The sandbox did not contain:

```text
rustup
cargo-clippy
```

and an appropriate `cargo-clippy` package was unavailable through the environment's package manager.

Therefore, project documentation must not claim that the current codebase has passed Clippy.

Before merging on a normal development machine, run:

```bash
cargo clippy --workspace --all-targets
```

Any resulting issues should be reviewed before declaring the milestone fully lint-clean.

---

# 11. Known Incomplete Components

The following functionality is intentionally **not implemented yet**.

Their absence should not be interpreted as a bug in the existing milestones.

---

## 11.1 Reasoning Engine

**Status: NOT IMPLEMENTED**

There is currently:

```text
No ReasoningEngine trait
No CloudReasoner
No Claude API integration
```

The Reasoning Engine is the **next required milestone**.

Its implementation requirements are defined in Section 12.

---

## 11.2 Action Router

**Status: NOT IMPLEMENTED**

There is currently no Action Router.

Therefore BELLA does not yet validate or dispatch actions proposed by a reasoning model.

At the current stage there are also no genuine proposed actions because the Reasoning Engine does not yet exist.

The Action Router should not be fabricated merely to complete a pipeline demonstration.

It belongs to its designated development phase.

---

## 11.3 Production Voice Subsystem

**Status: NOT IMPLEMENTED**

`DemoVoiceSubsystem` is architectural scaffolding.

The production Voice system does not yet contain:

```text
whisper.cpp
wake-word detection
TTS
Piper integration
production microphone pipeline
```

The existing demo correctly demonstrates the required permission and message-passing pattern.

The production Voice subsystem should preserve those architectural boundaries.

---

## 11.4 Production Context Builder

**Status: PARTIAL**

`DemoContextSubsystem` currently performs real memory operations:

```text
write episode
recall recent episodes
```

However, it does not yet construct a production prompt context.

Missing functionality includes:

```text
token budgeting
relevance filtering
context prioritization
bounded prompt assembly
```

The current implementation logs recalled memory rather than assembling a fully bounded reasoning context.

---

## 11.5 Semantic / Vector Search

**Status: DEFERRED**

Current:

```text
MemoryStore::search_content
```

uses SQLite:

```text
LIKE
```

substring matching.

This is not semantic search.

`sqlite-vec` was selected during the stack-design phase but has not yet been integrated.

Until that integration occurs, substring search should remain clearly documented as a temporary implementation.

---

## 11.6 Later Subsystems

The following major systems have not been started:

```text
Vision
Device Control
Automation
Plugin SDK
```

Other functionality belonging to later phases of the original 20-phase plan is likewise not yet implemented.

Development should continue according to the defined phase order.

---

# 12. Next Milestone — Reasoning Engine

The next implementation milestone is the **Reasoning Engine**.

The goal is to introduce BELLA's reasoning abstraction while preserving the ability to replace the cloud model with local inference in the future.

---

## 12.1 New Crate

Create:

```text
bella-reasoning
```

The crate should depend on:

```text
bella-common
```

and may depend on:

```text
bella-memory
```

if direct use of `Episode` or related memory types is necessary.

Dependencies should remain minimal and consistent with the existing architecture.

---

# 13. Reasoning Interface

The crate should define a:

```rust
ReasoningEngine
```

trait.

The implementation style should follow the existing async subsystem patterns used by:

```text
bella-core/src/subsystem_trait.rs
```

A conceptual interface is:

```rust
async fn reason(
    &self,
    context: AssembledContext,
) -> BellaResult<ProposedAction>;
```

The final API does not need to match this signature exactly.

Before defining the interface, inspect:

```text
Payload::AssembledContext
Payload::ProposedAction
```

inside:

```text
bella-common/src/message.rs
```

These existing payload types should guide the interface design.

If the current payload structures are insufficient for a real Reasoning Engine, they may be extended.

Changes should preserve clean subsystem boundaries rather than introducing reasoning-specific implementation details into unrelated crates.

---

# 14. Cloud Reasoner

The first production implementation of `ReasoningEngine` should be:

```text
CloudReasoner
```

The initial backend is expected to communicate with the Claude API over HTTPS.

A reasonable HTTP dependency is:

```text
reqwest
```

The workspace does not currently depend on it.

The HTTP implementation should provide:

```text
request construction
authentication
response parsing
error handling
timeout/error propagation
conversion into BELLA reasoning types
```

Provider-specific networking details should remain inside the cloud reasoning implementation.

The rest of BELLA should interact with the `ReasoningEngine` abstraction rather than directly depending on Claude-specific code.

---

# 15. API Key Handling

The long-term secret-management requirement is:

```text
libsecret / OS keyring
```

For the first Reasoning Engine implementation, reading the API key from an environment variable is acceptable as an explicitly temporary mechanism.

For example:

```text
BELLA_CLAUDE_API_KEY
```

If environment-variable loading is used, the implementation or documentation must state clearly that it is temporary.

Environment-variable storage should not be presented as the final secrets architecture.

Secret handling should later migrate to the operating system's secure credential storage according to the Phase 1 requirements.

---

# 16. Core Integration

After implementing the Reasoning Engine, integrate it into `bella-core`.

The current Context pipeline should evolve from:

```text
Voice
  ↓
Permission
  ↓
Context
  ↓
Memory
```

toward:

```text
Voice
  ↓
Permission
  ↓
Context
  ↓
Memory Recall
  ↓
Assembled Context
  ↓
Reasoning Engine
  ↓
Proposed Action
```

The Context subsystem should:

1. Record relevant interaction memory.
2. Recall required memory.
3. Construct the reasoning context.
4. Pass that context to the `ReasoningEngine`.
5. Receive a `ProposedAction`.
6. Forward or expose the result according to the currently available architecture.

---

# 17. Temporary Pipeline Boundary

The Action Router does not exist yet.

Therefore the Reasoning milestone should stop honestly at:

```text
ProposedAction
```

The resulting action may temporarily be logged.

Example conceptual boundary:

```text
Reasoning Engine
      ↓
ProposedAction
      ↓
LOG OUTPUT
```

Logging the proposed action is acceptable because the next subsystem does not yet exist.

An artificial Action Router should **not** be created simply to make the pipeline appear complete.

The real Action Router should be implemented during its designated phase.

---

# 18. Reasoning Engine Testing Requirements

The Reasoning Engine must include unit tests.

Existing BELLA crates establish the expected testing standard.

Tests should include normal behavior as well as deliberately adversarial or edge-case behavior.

Existing examples include testing:

```text
message bus backpressure
expired permission grants
ordering ties
```

The Reasoning Engine should maintain a comparable level of testing discipline.

---

# 19. Cloud API Testing

`cargo test` should not depend on the availability of a live Claude API.

Tests should not:

```text
consume API credits
require internet connectivity
require production credentials
fail because the external API is unavailable
```

The `ReasoningEngine` abstraction should therefore allow a test implementation.

Conceptually:

```rust
struct MockReasoner;
```

A mock implementation can return controlled `ProposedAction` values and allow the surrounding pipeline to be tested deterministically.

Production API communication and internal reasoning orchestration should remain separable enough to test independently.

---

# 20. Reasoning Milestone Acceptance Criteria

The Reasoning Engine milestone is complete only when all of the following are true:

```text
[ ] bella-reasoning crate exists

[ ] ReasoningEngine abstraction exists

[ ] CloudReasoner exists

[ ] Cloud API request/response handling works

[ ] Authentication mechanism exists

[ ] Temporary environment-variable secret handling is documented honestly

[ ] Context can invoke ReasoningEngine

[ ] ProposedAction is produced

[ ] ProposedAction reaches the current pipeline boundary

[ ] Unit tests exist

[ ] Tests do not require the live cloud API

[ ] cargo build succeeds

[ ] cargo test succeeds

[ ] cargo run --bin bellad has been executed

[ ] Runtime behavior has been observed
```

The milestone should not be marked complete solely because the code compiles.

---

# 21. Documentation Recovery

The following documentation is expected to exist:

```text
docs/architecture.md
docs/adr/
docs/adr/0001-original-brief.md
```

Some earlier design material was originally produced during development discussions and may not have been persisted to the repository.

This includes:

```text
Phase 1 — Requirements
Phase 2 — Architecture
Phase 3 — Stack
Phase 4 — Repository Structure
```

If `docs/architecture.md` or required ADR content is missing, the documentation should be reconstructed from:

1. This engineering handoff
2. Existing repository structure
3. Existing working code
4. Available development history

The reconstructed documentation must reflect the actual implementation.

Working code and passing tests take precedence over stale design prose.

Architectural information should not remain dependent on development chat history.

---

# 22. Architectural Change Policy

Existing architectural decisions should not be casually reversed during feature implementation.

Examples include:

```text
Rust core daemon
MessageBus-based subsystem communication
Permission-gated privileged operations
SQLite memory storage
Cloud-first ReasoningEngine
Future local reasoning implementation
Phase-driven development
```

If one of these decisions becomes inappropriate, the change should be intentional and documented.

A significant architectural change should include:

```text
Reason for change
Alternatives considered
Migration impact
Dependency impact
Security impact
Testing impact
```

Where appropriate, create a new Architecture Decision Record.

---

# 23. Dependency Discipline

New dependencies should be introduced only when they solve a concrete requirement.

Before adding a dependency, consider:

```text
Does an existing workspace dependency already solve this?

Does it introduce unnecessary runtime weight?

Does it break the minimum supported Rust version?

Does it violate subsystem boundaries?

Does it introduce unnecessary unsafe code?

Does it create a security-sensitive dependency?

Can the dependency be isolated inside one crate?
```

Dependencies should remain local to the subsystem that requires them whenever possible.

---

# 24. Error Handling

Production code should propagate meaningful errors through:

```text
BellaError
BellaResult
```

Silent failures should be avoided.

Errors involving external systems should preserve enough information for debugging while avoiding exposure of secrets.

Examples include:

```text
network failure
API authentication failure
invalid API response
database failure
permission denial
message bus failure
subsystem startup failure
```

Error handling should remain consistent with the common error model rather than introducing unrelated crate-specific error conventions without a clear reason.

---

# 25. Security Requirements for Future Subsystems

Any subsystem that accesses privileged resources must integrate with the permission system.

Examples include future:

```text
Voice
Vision
Device Control
Automation
Plugin execution
Filesystem operations
Shell execution
```

The required flow is conceptually:

```text
Subsystem requests privileged operation
                ↓
PermissionSystem::check()
                ↓
        Allow / Deny
                ↓
        Audit entry recorded
                ↓
Operation executes only if allowed
```

Subsystems must not bypass this flow for convenience.

---

# 26. Message Bus Boundary

Subsystem interaction should remain message-oriented.

Preferred:

```text
Subsystem A
    ↓
Envelope + Payload
    ↓
MessageBus
    ↓
Subsystem B
```

Avoid introducing architecture such as:

```text
Subsystem A
    ↓
direct reference
    ↓
Subsystem B internal implementation
```

unless an explicit architectural decision establishes a justified exception.

This boundary is essential for:

```text
modularity
testing
fault isolation
future plugin support
subsystem replacement
local/cloud implementation swapping
```

---

# 27. Supervisor Expectations

Long-running subsystems should operate under the established supervision model.

The Supervisor currently provides restart behavior for panicking subsystem tasks.

Current policy:

```text
Maximum automatic restarts: 5
Backoff: 500 ms
```

Future subsystem implementations should integrate with this model rather than creating independent unmanaged long-running tasks without architectural justification.

---

# 28. Definition of Done

A BELLA development task is not considered complete merely because an implementation exists.

The expected completion standard is:

```text
Implementation complete
        ↓
Build successful
        ↓
Tests successful
        ↓
Runtime execution successful
        ↓
Behavior observed
        ↓
Documentation updated
```

At minimum:

```bash
cargo build
cargo test
cargo run --bin bellad
```

should be executed when applicable.

If a verification step cannot be executed because of an environment limitation, that limitation must be documented rather than reporting the verification as successful.

---

# 29. Current Development Boundary

At the time of this handoff, BELLA has working foundations for:

```text
Common contracts
Message bus
Permission system
Memory system
Subsystem abstraction
Supervisor
Cross-subsystem communication
Memory persistence
Memory recall
```

The architecture has been demonstrated through the current pipeline:

```text
Demo Voice
    ↓
Permission
    ↓
Demo Context
    ↓
Memory
```

The next boundary is:

```text
Context
    ↓
Reasoning Engine
    ↓
ProposedAction
```

The project should advance from this point rather than rebuilding already validated foundations.

---

# 30. Immediate Development Objective

The immediate engineering objective is:

> **Implement the Reasoning Engine while preserving BELLA's existing subsystem boundaries, security model, testing standards, and future ability to support local inference.**

The implementation sequence is:

```text
Create bella-reasoning
        ↓
Define ReasoningEngine
        ↓
Review AssembledContext / ProposedAction
        ↓
Implement CloudReasoner
        ↓
Implement temporary secure-key loading boundary
        ↓
Integrate with Context
        ↓
Produce ProposedAction
        ↓
Add deterministic tests
        ↓
cargo build
        ↓
cargo test
        ↓
cargo run --bin bellad
        ↓
Observe runtime behavior
        ↓
Update documentation
```

Once these requirements are satisfied, the Reasoning Engine milestone can be considered complete and development can proceed to the next phase defined by the BELLA roadmap.

---

# Final Engineering Principle

BELLA should remain a system whose features are **actually implemented, actually tested, and actually executable**.

The project favors:

```text
working code over impressive scaffolding
explicit limitations over fake completeness
stable interfaces over tightly coupled subsystems
auditable permissions over ambient authority
documented decisions over forgotten assumptions
runtime verification over compilation-only confidence
```

Every new subsystem should preserve those principles as BELLA grows.
