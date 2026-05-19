# Contributing to error-forge

Thanks for considering a contribution. This document describes
the local workflow, the CI gates a PR has to pass, and the
release-discipline rules that contributors should be aware of.

## Quick start

```bash
git clone https://github.com/jamesgober/error-forge
cd error-forge

cargo build --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

If all four pass on your machine, your local environment
matches what CI will check.

## Toolchain

- **Stable Rust**, currently `1.95+` for day-to-day work.
- **MSRV: `1.81.0`**. The crate must continue to build on the
  exact `1.81.0` toolchain — a dedicated CI job verifies this on
  every push.

The `Cargo.toml` `rust-version` field is the source of truth.
Any change to it is a minor-version bump minimum
(see [`docs/STABILITY.md`](docs/STABILITY.md) §MSRV).

## CI gates a PR must pass

A PR is mergeable when **every** check below is green on every
matrix entry. Run them locally before pushing:

```bash
# Build
cargo build --workspace --no-default-features
cargo build --workspace --all-features

# Test
cargo test --workspace --all-features

# Lints
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo fmt --all -- --check

# Docs
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

# MSRV
cargo +1.81 build --workspace --all-features

# Security advisory scan
cargo audit
```

The CI workflow runs the build / test step across nine feature
combinations on `ubuntu-latest`, `macos-latest`, and
`windows-latest`. The `clippy`, `docs`, `msrv`, and `audit`
jobs run once on Linux.

## Code style

- **Formatting:** `rustfmt` defaults. `cargo fmt` before commit.
- **Lints:** the crate-root `#![deny(...)]` lint block (in
  `src/lib.rs`) catches `unwrap_used`, `expect_used`,
  `print_stdout`, `dbg_macro`, `unreachable`, `todo`,
  `unimplemented`, `missing_docs`, `unused_must_use`,
  `unused_results`, and several others. Library code that
  drifts from these rules fails the build.
- **`unsafe`:** the crate has **zero** `unsafe` blocks. Adding
  one requires explicit justification in the PR description and
  a `// SAFETY:` doc comment.

## Banned terms

The following terms are not allowed in committed prose
(`README.md`, `docs/`, doc comments, CHANGELOG entries):

- `comprehensive`
- `robust`
- `seamless`
- `leverage`

They are reach-words; concrete language describes behaviour
better. Use specific verbs (`covers`, `verifies`, `accepts`,
`uses`) instead.

A `cargo audit` job runs on every push.

## Documentation expectations

- Every `pub fn` / `pub struct` / `pub enum` / `pub trait`
  must carry rustdoc with at least one `# Example` block where
  the example would compile. Use `# Example` headings, not
  `# Examples` (project convention).
- Doc tests must compile and run. `ignore` is reserved for
  cases that *cannot* run (e.g. known parser ambiguities,
  feature-gated code that the doc-test harness can't compile).
  Each `ignore` must carry an inline explanation.
- Banned terms (see above) are checked against committed
  files; CI does not auto-block but reviewers will.

## Test expectations

- Public-API changes require new tests. Either a unit test
  inside the source module, or an integration test under
  `tests/`.
- Tests that genuinely cannot run on all platforms use
  `#[cfg(target_os = "...")]` and document why.
- Property-style invariants are welcome but not required.
- Tests must follow the
  `test_<subject>_<condition>_<expected>` naming convention
  for new integration tests (existing tests are
  grandfathered).

## Release procedure

Release procedure is documented in [`RELEASING.md`](RELEASING.md).
Contributors generally do not need to run it; maintainers do.

Briefly: CHANGELOG, version bump, tag, publish derive sub-crate
first, then main crate. See `RELEASING.md` for the full
checklist.

## Reporting bugs

Open an issue at
<https://github.com/jamesgober/error-forge/issues>. Include:

- The version of `error-forge` you're using.
- The version of Rust you're using
  (`rustc --version --verbose`).
- A minimal repro.
- What you expected vs. what happened.

For security-relevant issues, see [`SECURITY.md`](SECURITY.md)
instead — do **not** open a public issue.

## Proposing features

Open an issue first describing the proposed feature, its use
case, and the public-API shape you have in mind. A PR is more
likely to land if it has prior agreement on shape.

## Code of conduct

Be respectful, assume good faith, and focus on the technical
merits. Personal attacks, harassment, or discriminatory
language are not tolerated.

## Licence

By contributing, you agree that your contributions will be
licensed under the Apache License 2.0, the same licence as the
rest of the project.
