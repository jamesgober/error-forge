# Security Policy

## Supported Versions

The `1.x` line is supported. Security fixes ship as patch
releases (`1.x.Y`) within the line; see
[`docs/STABILITY.md`](docs/STABILITY.md) for the binding SemVer
contract.

Older lines (`0.x`) are not supported. Upgrade to the latest
`1.x` for security fixes. See
[`docs/migration.md`](docs/migration.md) for migration guides.

| Version | Supported          |
|---------|--------------------|
| `1.x`   | ✓ (current)        |
| `0.9.x` | ✓ until 2026-12-31 |
| `< 0.9` | ✗                  |

## Reporting a Vulnerability

If you find a security vulnerability in `error-forge` or
`error-forge-derive`, please **report it privately** rather than
opening a public GitHub issue.

### Preferred channel

Email **me@jamesgober.com** with the subject line
`[error-forge security]` plus a one-line summary.

Include in the report:

- The affected version(s).
- A minimal repro (code snippet or test case).
- Your assessment of severity and exploitability.
- Whether you intend to coordinate a CVE / GHSA filing.

### Response timeline

- **Acknowledgement:** within 72 hours.
- **Triage and severity assessment:** within 7 days.
- **Patch release:** as soon as a fix is verified, typically
  within 14 days for high-severity issues.
- **Public disclosure:** coordinated with the reporter, after
  the patch is published on crates.io.

### What qualifies

Security-relevant issues include but are not limited to:

- Memory safety bugs (use-after-free, double-free, OOB access)
  — even though `error-forge` has no `unsafe` of its own, a bug
  that triggers UB in `parking_lot`, `pastey`, or the standard
  library through our usage qualifies.
- Information leakage through panic payloads, error messages,
  or `Display` / `Debug` impls.
- Denial of service via unbounded allocation, infinite loops,
  or lock-poisoning cascades (the `CircuitBreaker` uses
  `parking_lot::Mutex` specifically to avoid the latter).
- Logic bugs in the `recovery` primitives that produce
  incorrect retry / circuit-breaker behaviour under
  adversarial input.

If you're not sure whether something qualifies, err on the
side of reporting privately.

## Verification

Every push to `main` runs:

- `cargo audit` against the RustSec advisory database
  (catches deprecated / vulnerable transitive deps).
- The full CI matrix described in
  [`README.md`](README.md#quality-bar).

`cargo audit` flags both vulnerabilities (`vulnerability`) and
unmaintained crates (`unmaintained` warning). Either status
triggers a release within the timelines above if the affected
dep cannot be removed or worked around.

## Acknowledgements

Reporters who help us fix issues responsibly are credited in
the release-notes file (`.dev/release/vX.Y.Z.md`) and in the
GitHub release announcement, unless they request anonymity.
