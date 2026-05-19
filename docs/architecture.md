# error-forge — Architecture and Design Rationale

> Why each major piece looks the way it does. Pairs with
> [`docs/API.md`](API.md) (what the surface is) and
> [`docs/STABILITY.md`](STABILITY.md) (what we promise).

## High-level model

`error-forge` is built around three concepts:

1. **`ForgeError` trait** — every error in the system carries
   stable metadata (kind, retryable, fatal, status code, exit
   code, captions, dev / user messages) on top of
   `std::error::Error`. This is what separates `error-forge` from
   "just `thiserror` plus `anyhow`": the metadata is part of the
   contract, not an ad-hoc per-project convention.

2. **Wrappers** — `CodedError<E>`, `ContextError<E, C>`, and
   `ErrorCollector<E>` decorate or accumulate underlying errors
   while preserving the `ForgeError` contract. The wrappers
   compose: `Vec<ContextError<CodedError<AppError>, &str>>` is
   a legal type.

3. **Recovery primitives** — `RetryPolicy`, `CircuitBreaker`,
   and the `Backoff` trait sit on top of `ForgeError` and react
   to its metadata (`is_retryable` decides retry; circuit-breaker
   counts failures from the same underlying call site).

The crate root re-exports each layer; users typically pick the
subset they need.

## Why a `ForgeError` *trait* instead of a single enum

A single error enum (`error_forge::Error { ... }`) would be
either too small (every project has its own variants) or too
large (a god-enum). The trait approach lets each project define
its own variants while still benefiting from the surrounding
machinery (collectors, recovery, registry, formatting).

`AppError` is the "small project starter" — covers `Config`,
`Filesystem`, `Network`, `Other`. Larger projects move to
`define_errors!` or `#[derive(ModError)]` enums.

## Why `define_errors!`, `#[derive(ModError)]`, AND `group!`

Three layers, each addressing a different ergonomic point:

- **`define_errors!`** (declarative macro). For new error
  enums, written from scratch. Generates constructors,
  `Display`, `Error`, `ForgeError`. Good when you're starting
  fresh.

- **`#[derive(ModError)]`** (proc macro, optional via the
  `derive` feature). For enums you've already written and want
  to annotate. Doesn't require macro-style declaration. Good
  for projects that already have a `thiserror`-shaped enum and
  want `ForgeError` on top.

- **`group!`** (declarative macro). For composing multiple
  existing error types into a parent enum. Doesn't generate
  metadata — it delegates each method to the wrapped variant's
  own `ForgeError` impl. Good at module boundaries where
  several sub-system errors merge.

Each macro exists because the others would be awkward for its
use case. `define_errors!` is too verbose for an existing enum;
`#[derive]` can't generate from-scratch enums; `group!` would
be silly to use when you don't have multiple existing types to
group.

## The hook system

`error-forge`'s constructors auto-fire a registered hook on every
error creation. This is the operational integration point —
register once at startup, get notified of every error in the
process.

### Why a single global hook

Multiple-hook designs (each subsystem registers its own) add
complexity for marginal benefit. Users who want fan-out
register a single hook that internally dispatches. Single-hook
keeps the registration surface small and the cost predictable
(`OnceLock::get` is one atomic load per error).

### Why a `Box<dyn Fn>` callback (since `1.0`)

The `0.9.x` line stored `fn(ErrorContext)` — function pointer
only, no closure capture. Users wanting to log into a
state-carrying writer had to use `static mut` or a separate
global. `1.0` widens to `Box<dyn Fn + Send + Sync + 'static>`,
which costs one extra indirection per call (negligible on the
error path) and lets closures capture thread-safe state.

### Why `OnceLock`

A `OnceLock` is enough — hooks are register-once, never replace.
A `RwLock<Option<...>>` would allow dynamic re-registration but
adds lock-acquisition cost to every error construction. The
choice prioritises the hot path.

## `ConsoleTheme` as `&'static str` fields

ANSI escape codes are static strings. Storing them as `String`
allocates 8 heap blocks per theme construction; `&'static str`
allocates none. The `1.0` `ConsoleTheme` is `Copy`-compatible in
spirit (private fields prevent `derive(Copy)` directly, but
that's a future minor-version addition if useful).

`print_error` caches the default theme in a process-wide
`OnceLock` so the terminal-capability check
(`std::io::IsTerminal` + env-var probes) runs at most once.

## Recovery primitives are synchronous

`RetryExecutor` uses `std::thread::sleep`. This is deliberate:
async retry semantics depend on the executor (tokio, async-std,
smol), and `error-forge` is otherwise runtime-agnostic. Coupling
to one would force a feature-flag matrix; refusing to handle
async retry keeps the crate simple.

Users who need async retry compose with their runtime: keep
`error-forge` for modelling and classification, and wrap retry
behaviour with their async runtime's retry helper.

The blocking sleep is fine for worker-thread code, batch jobs,
and CLI tools — all of which are valid `error-forge` use cases.

## `CircuitBreaker` uses `parking_lot::Mutex` (since `1.0`)

`0.9.x` used `std::sync::Mutex` with `.unwrap()` on every
`lock()`. A panic anywhere inside the `execute(|| ...)` closure
would poison the mutex, and every subsequent `execute` would
panic at unwrap. That's the opposite of what an error-handling
crate should ship.

`parking_lot::Mutex` has no poisoning. The cost is one extra
runtime dep, which is acceptable for the correctness guarantee.

## `CodedError::new` is now allocation-only (since `1.0`)

`0.9.x`'s `CodedError::new` auto-registered the code in the
global registry if not already present. For codes that appear
across many error paths, the first occurrence per code per
process paid a write-lock on the registry. `1.0` drops the
auto-register and documents pre-registration at startup as the
intended pattern. Hot-path cost is now one `String` allocation
(the code) and zero locking.

The trade-off: callers who relied on auto-registration must add
explicit `register_error_code(...)` calls at process start.
Documented in [`docs/migration.md`](migration.md).

## `TypeId`-free dispatch

Unlike `mod-events`, `error-forge` does no `TypeId`-keyed dispatch.
Every operation works on a concrete `T: ForgeError` via static
dispatch (generic functions, monomorphised at compile time) or
on `&dyn ForgeError` via dynamic dispatch (one vtable call). No
`TypeId`-keyed hashmaps; no specialised hashers needed.

The performance story is "fast because there's nothing to be
slow about" — not because of clever data-structure choices.

## Memory profile

- **`AppError`**: ~64 bytes for the largest variant
  (`Filesystem` with `Option<PathBuf>`, `io::Error`, plus the
  three small `bool` / `u16` fields).
- **`CodedError<E>`**: `E` + `String` (code) + 3 small fields.
- **`ContextError<E, C>`**: `E` + `C`.
- **`ErrorCollector<E>`**: `Vec<E>`, allocates lazily.
- **`ConsoleTheme`**: 8 `&'static str` fields. 64 bytes on a
  64-bit system; zero heap.
- **`CircuitBreaker`**: `Arc<Mutex<CircuitBreakerInner>>` +
  name `String`. The inner state is a config + `Vec<Instant>`
  of recent failures.
- **Global hook**: `OnceLock<Box<dyn Fn>>`. One pointer + one
  vtable per process.
- **Global registry**: `OnceLock<ErrorRegistry>` containing
  `RwLock<HashMap<String, ErrorCodeInfo>>`. Lazily initialised
  on first use.

## Public surface footprint

Quantified in [`docs/API-FREEZE-AUDIT.md`](API-FREEZE-AUDIT.md).
The headline numbers:

- 1 always-available trait (`ForgeError`).
- 1 always-available enum (`AppError`).
- 4 wrapper types (`ContextError`, `CodedError`, `ErrorCollector`,
  with `AsyncForgeError` under `feature = "async"`).
- 3 backoff types + `CircuitBreaker` + `RetryPolicy` /
  `RetryExecutor`.
- 2 declarative macros (`define_errors!`, `group!`).
- 1 proc macro (`#[derive(ModError)]`) under
  `feature = "derive"`.

## What's deliberately NOT here

- **Source-snippet diagnostic rendering** (compiler-style error
  reports with labels). That's `miette` territory.
- **Async retry primitives.** See "Recovery is synchronous"
  above.
- **A `RetryConfig` builder file format.** Configuration is
  builder-style on `RetryPolicy` / `CircuitBreakerConfig`. No
  TOML / YAML parsing.
- **A panic-to-error converter.** `install_panic_hook` formats
  panics for display; it does not convert them into
  `ForgeError` values. That conversion would erase context.

## Versioning posture

The `1.x` line locks the public surface. Specifics in
[`docs/STABILITY.md`](STABILITY.md). Performance is not part of
the contract; `1.x.Y` patches may change internal data structures
freely.
