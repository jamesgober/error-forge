## Summary

<!-- Short description of what this PR changes and why. -->

## Type of change

<!-- Tick all that apply. -->

- [ ] Bug fix
- [ ] New feature (additive only)
- [ ] Breaking change (requires a major version bump per
      [`docs/STABILITY.md`](../docs/STABILITY.md))
- [ ] Documentation only
- [ ] CI / build / tooling

## Public-surface impact

<!-- Anything externally visible? New `pub fn`, new variant on a
     non-exhaustive enum, new field on a non-exhaustive struct,
     new feature flag, changed MSRV, etc. If none, say "none". -->

## CHANGELOG

- [ ] I added an entry under `## [Unreleased]` in `CHANGELOG.md`
      describing the user-visible effect of this change, per
      [`.dev/DIRECTIVES.md`](../.dev/DIRECTIVES.md) §D-1.

## Local verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`
- [ ] `cargo build --workspace --no-default-features`
- [ ] `cargo build --workspace --all-features`
- [ ] `cargo +1.81 build --workspace --all-features` (MSRV)
- [ ] `cargo test --workspace --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`
- [ ] `cargo audit` (if dep tree changed)

## Tests

<!-- New tests added? Existing tests cover the change? If
     not, justify why. -->

## Documentation

<!-- Any documentation updated? Public `pub fn` / `pub struct`
     additions need rustdoc with `# Example`. -->

## Additional context

<!-- Anything reviewers should know up front. -->
