# error-forge vs. the Rust error-handling ecosystem

> Honest side-by-side. When `error-forge` is the right tool, and
> when something else is.

`error-forge` is one of several error-handling crates available
on crates.io. The headline tradeoffs:

| Choosing between... | If you want...                                                  | Pick                       |
|---------------------|-----------------------------------------------------------------|----------------------------|
| `error-forge` vs `anyhow`        | A single dyn-error type for application code; type-erased | `anyhow`                   |
| `error-forge` vs `anyhow`        | Stable per-error metadata (kind, retryable, status code)  | `error-forge`              |
| `error-forge` vs `thiserror`     | Just `#[derive(Error)]` on a custom enum; zero runtime    | `thiserror`                |
| `error-forge` vs `thiserror`     | A `Error` derive plus error codes, recovery primitives    | `error-forge`              |
| `error-forge` vs `miette`        | Beautiful diagnostic reports with source-snippets         | `miette`                   |
| `error-forge` vs `miette`        | A non-`miette`-shaped error model with HTTP-status helpers| `error-forge`              |
| `error-forge` vs `snafu`         | Snapshotted source-with-context using `context selectors` | `snafu`                    |
| `error-forge` vs `snafu`         | Builder-style errors with extension traits                | `error-forge`              |
| `error-forge` vs `eyre`          | An `anyhow` work-alike with a custom report type          | `eyre`                     |

The rest expands on each.

---

## `error-forge` vs `anyhow`

[`anyhow`](https://crates.io/crates/anyhow) is the standard
choice for application-level error handling. One catch-all
`anyhow::Error` type, automatic conversion from any
`E: std::error::Error`, context chaining, easy `?` propagation.

### Things `anyhow` does that `error-forge` doesn't

- **Type-erased single error type.** `anyhow::Error` wraps any
  error implementor without you having to enumerate variants.
- **Backtrace integration is automatic** (under the `backtrace`
  feature) — `error-forge` exposes a `backtrace()` method on
  `ForgeError` but the trait itself doesn't capture backtraces
  for you.
- **Ubiquitous ecosystem support.** Hundreds of libraries
  return `anyhow::Result` directly; `error-forge` is its own
  shape.

### Things `error-forge` does that `anyhow` doesn't

- **Stable per-error metadata.** Every `ForgeError` carries
  `kind`, `caption`, `is_retryable`, `is_fatal`, `status_code`,
  `exit_code`. `anyhow` errors carry only their `Display` and
  `Error::source` chain.
- **HTTP-status routing out of the box.** `status_code(&self) -> u16`
  is part of the trait. Useful for web frameworks that map
  errors to HTTP responses.
- **Operational hooks.** A registered hook fires on every error
  construction. `anyhow` has no equivalent (you build your own
  with `with_context`).
- **Recovery primitives.** `RetryPolicy`, `CircuitBreaker`,
  three backoff strategies in the same crate. `anyhow` is
  modelling-only.
- **Error codes with documentation URLs.** `CodedError<E>` and
  `ErrorRegistry` are core surface.

### When to pick which

- `anyhow` if your error story is "wrap anything, propagate up,
  print at top-level."
- `error-forge` if your error story is "categorise, route,
  retry, log, present to users."

You can use both in the same project — `anyhow::Error` wraps
any `ForgeError` cleanly via the `From<E: Error>` impl.

---

## `error-forge` vs `thiserror`

[`thiserror`](https://crates.io/crates/thiserror) is the
standard choice for library-level custom error enums.
`#[derive(Error)]` generates `Display` and `Error` impls from
attributes. Zero runtime cost.

### Things `thiserror` does that `error-forge` doesn't

- **Pure compile-time.** No runtime dependency at all.
  `error-forge` has runtime deps (`thiserror`, `pastey`,
  `parking_lot`).
- **Standard error-attribute syntax.** Every Rust dev knows
  `#[error("...")]`.
- **Zero metadata.** Just `Error` + `Display`. Nothing more,
  nothing less.

### Things `error-forge` does that `thiserror` doesn't

- **`ForgeError` trait** gives every variant
  `is_retryable` / `is_fatal` / `status_code` / `exit_code`
  metadata that operational tooling cares about.
- **`#[derive(ModError)]` proc macro** + `define_errors!` /
  `group!` declarative macros provide three layers of
  ergonomics on top of `thiserror`.
- **Built-in `ErrorCollector`, `ContextError`, `CodedError`,
  registry, recovery primitives**, none of which `thiserror`
  attempts.

### When to pick which

- `thiserror` for library crates exporting a custom error
  enum that downstream users will wrap.
- `error-forge` for application code or libraries that want to
  ship operational metadata with every error variant.

`error-forge` already depends on `thiserror` internally — they
compose, they don't compete.

---

## `error-forge` vs `miette`

[`miette`](https://crates.io/crates/miette) focuses on
beautiful, terminal-style diagnostic reports — labels, source
spans, related-info pointers, severity colours, the works.

### Things `miette` does that `error-forge` doesn't

- **Source-snippet rendering with labels.** Show the exact
  byte range in a file that caused the error.
- **Help / advice prompts.** Errors can carry "did you mean"
  hints rendered inline.
- **Diagnostic codes** (similar to `error-forge`'s `CodedError`
  but with `miette`'s rendering integrated).
- **Compiler-style error formatting**, ideal for parsers,
  language tools, query engines.

### Things `error-forge` does that `miette` doesn't

- **Programmatic metadata** (`is_retryable`, `status_code`)
  intended for tooling that *acts* on errors, not just
  displays them.
- **Recovery primitives.** `miette` is presentation-focused;
  retry and circuit-breaker behaviour are not in scope.
- **Lower dependency surface.** `miette` brings in a number of
  rendering deps (`owo-colors`, `textwrap`, etc.) by default.

### When to pick which

- `miette` if your errors are for humans reading a terminal
  (compilers, query engines, build tools, dev-tooling CLI).
- `error-forge` if your errors are for tooling that classifies,
  retries, logs, or maps to HTTP responses.

The two can compose — define a `ForgeError` enum that also
derives `miette::Diagnostic` if you want both surfaces.

---

## `error-forge` vs `snafu`

[`snafu`](https://crates.io/crates/snafu) is `thiserror`-shaped
with a stronger emphasis on context chaining via "context
selectors" — generated helper structs that wrap any error
along with explicit named fields.

### Things `snafu` does that `error-forge` doesn't

- **Context selectors.** Every variant gets a generated builder
  that turns any error into a fully-annotated variant via
  `.context(ContextSelector { ... })`.
- **Visibility / module discipline.** Snafu has strong opinions
  about which errors are "internal" vs "external" to a module.
- **Backtrace integration on every variant** by default.

### Things `error-forge` does that `snafu` doesn't

- **Metadata helpers** (`is_retryable`, etc.) — `snafu` cares
  about the error's *origin*; `error-forge` cares about what
  the error *means* for operational decisions.
- **`ErrorCollector`** for accumulating multiple errors before
  surfacing them as one — `snafu` doesn't have a parallel.
- **Recovery primitives** — same point as `anyhow`.

### When to pick which

- `snafu` for libraries with rich internal error topology and a
  strict context-chaining discipline.
- `error-forge` when your operational tooling is the primary
  consumer of error metadata.

---

## `error-forge` vs `eyre`

[`eyre`](https://crates.io/crates/eyre) is an `anyhow` fork
with a pluggable report type. Most of the comparison points
mirror the `anyhow` discussion above.

### Things `eyre` does that `error-forge` doesn't

- **Custom report types.** Plug in `color-eyre` for terminal
  colouring, or your own custom report formatter. `error-forge`
  has `ConsoleTheme` but it's not as plug-and-play.
- **`Section`** trait for incremental context — looser than
  `error-forge`'s `ContextError`.

### Things `error-forge` does that `eyre` doesn't

- Same set as the `anyhow` comparison: stable metadata,
  recovery primitives, error-code registry.

### When to pick which

- `eyre` (or `color-eyre`) when you want `anyhow` ergonomics
  with a slicker terminal report.
- `error-forge` when you need metadata + recovery, not just
  reporting.

---

## Honest non-strengths of `error-forge`

The cases where `error-forge` is clearly *not* the right
choice:

1. **You want zero runtime deps.** `error-forge` has three
   always-on deps (`thiserror`, `pastey`, `parking_lot`) and
   optional deps behind opt-in features. `thiserror` alone has
   zero runtime cost; choose it if dep footprint is the
   constraint.

2. **You want compiler-style diagnostic reports.** `miette` is
   the right tool. `error-forge`'s `ConsoleTheme::format_error`
   produces a single-error human-readable message, not a
   multi-source diagnostic with span labels.

3. **You're writing a leaf library that should be neutral.**
   Returning `error_forge::AppError` from a leaf-crate API
   imposes the entire `error-forge` model on downstream users.
   Use `thiserror` (or `std::io::Error`-like raw error
   handling) for library APIs, and reserve `error-forge` for
   the application layer.

4. **You need async recovery.** `error-forge`'s `RetryExecutor`
   uses `std::thread::sleep`, which is blocking. For async retry
   loops, use a tokio / async-std-aware retry crate.

If none of those apply, `error-forge` gives you a single
crate with metadata, codes, context, collectors, recovery, and
formatting in one shape. Most application code that reaches for
"I need an error type with operational metadata" lives
comfortably inside that surface.
