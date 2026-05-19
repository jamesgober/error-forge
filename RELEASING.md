# Releasing error-forge

This document codifies the release procedure. Following it
guarantees that the git tag, `Cargo.toml` version, CHANGELOG
heading, release-note filename, and crates.io publication stay
in sync.

The procedure is the same for `error-forge` and the proc-macro
sub-crate `error-forge-derive` — they are versioned together,
and the derive crate must publish first because the main crate
depends on a specific `error-forge-derive` version.

## Versioning

`error-forge` follows [Semantic Versioning 2.0.0](https://semver.org/),
with the contract pinned in [`docs/STABILITY.md`](docs/STABILITY.md):

- **Patch (`1.x.Y`)** — bug fixes, doc improvements, internal
  performance work, test additions. No new public items.
- **Minor (`1.x.0`)** — pure additions to the public surface,
  new opt-in features, new variants on `#[non_exhaustive]`
  enums, new fields on `#[non_exhaustive]` structs, MSRV bumps.
- **Major (`X.0.0`)** — removal / rename / signature change of
  any public symbol, removing `#[non_exhaustive]`, adding a
  required runtime dep, breaking the documented migration path.

Pre-release suffixes use `-alpha.N`, `-beta.N`, `-rc.N`.

## Pre-release checklist

Before tagging:

- [ ] All planned work for the release is merged to `main`.
- [ ] `CHANGELOG.md` has a complete entry under the new version
      heading. Move items from `[Unreleased]`; reset
      `[Unreleased]` to empty placeholders.
- [ ] `Cargo.toml` `version` in **both** `./Cargo.toml` and
      `./error-forge-derive/Cargo.toml` matches the new version.
- [ ] The main crate's `[dependencies]` entry for
      `error-forge-derive` matches the new version exactly.
- [ ] `README.md` install snippet shows the new version.
- [ ] `.dev/release/vX.Y.Z.md` exists and describes the release.

## Local verification gate

Run every check below and confirm all green:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo build --workspace --no-default-features
cargo build --workspace --all-features
cargo +1.81 build --workspace --all-features
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo audit
cargo publish -p error-forge-derive --dry-run
cargo publish -p error-forge --dry-run
```

Note: the main-crate dry-run will fail with
`failed to select a version for the requirement
error-forge-derive = "^X.Y.Z"` until the derive sub-crate is
actually published. That's expected — dry-run the main crate
*after* the derive crate is on crates.io.

## Tagging and pushing

```bash
git add Cargo.toml error-forge-derive/Cargo.toml \
        README.md CHANGELOG.md src/ docs/ benches/ examples/ \
        .dev/release/vX.Y.Z.md
git commit -m "Release vX.Y.Z"

git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

The git tag, `Cargo.toml` version, and the CHANGELOG heading
**MUST** match exactly. CI rejects releases where they
diverge.

## Publishing to crates.io

**The derive sub-crate must publish first** because the main
crate's `Cargo.toml` declares
`error-forge-derive = { version = "X.Y.Z", ... }` — that
version must be available on crates.io before the main crate
can publish.

```bash
# 1. Publish derive sub-crate
cargo publish -p error-forge-derive

# 2. Wait ~30 seconds for the crates.io sparse index to update.
#    (Verify via `cargo search error-forge-derive | head -3`.)

# 3. Publish main crate
cargo publish -p error-forge
```

If step 3 fails with "candidate versions found which didn't
match", the index hasn't caught up. Wait another minute and
retry.

## Post-release

- [ ] GitHub release created with body lifted from
      `.dev/release/vX.Y.Z.md`. Title: `vX.Y.Z — <summary>`.
- [ ] crates.io page renders (`https://crates.io/crates/error-forge`).
- [ ] docs.rs build succeeds
      (`https://docs.rs/error-forge/X.Y.Z`).
- [ ] CI is green on the release commit.

## Recovery procedures

### Publish failed mid-flight

The derive crate published but the main crate failed. **The
derive crate cannot be un-published** (crates.io only allows
yanking). Options:

1. Wait for the index to catch up, retry the main-crate publish.
2. If the derive crate has a real bug, yank it
   (`cargo yank --version X.Y.Z -p error-forge-derive`) and
   release a new patch version with the fix.

### Wrong version published

Yank, fix, release new version. **Never** delete tags or rewrite
history on `main`. Tagged versions are immutable.

```bash
cargo yank --version X.Y.Z -p error-forge
cargo yank --version X.Y.Z -p error-forge-derive
# Then bump and re-release as X.Y.(Z+1).
```

### CI was green locally but red on push

The most common causes:

- The `Cargo.lock` is format v4 (Rust 1.78+) but you tested on
  the `1.81.0` MSRV which can parse it; pushing to a fresh
  runner without your local cache exposes a difference. The
  MSRV job catches this.
- A platform-specific test that passed on your dev OS fails on
  another. The matrix catches this.
- A test depends on a tool not installed on the runner. Inspect
  the failed job logs (`gh run view <RUN_ID> --log-failed`).

Investigate, push a fix, and re-tag if necessary (only if the
new tag has not yet been pushed).

## Pre-1.0 history

Pre-1.0 release notes live in `.dev/release/v0.x.y.md`.
Pre-1.0 releases occasionally skipped this ceremony; `1.0.0`
formalises it.
