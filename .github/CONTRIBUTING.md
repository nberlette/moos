# Contributing to moos

Thanks for taking the time to improve **moos**! This crate is a low-level,
performance-focused library, so every bug report, benchmark, and patch helps us
deliver reliable primitives for `no_std` environments.

## Quick start checklist

1. Read the [Code of Conduct](./CODE_OF_CONDUCT.md) and be excellent to one
   another.
2. Discuss substantial changes through an issue or discussion thread before
   opening a pull request (PR).
3. Keep PRs focused, documented, and accompanied by tests and benchmarks when
   applicable.

---

## Reporting issues

- Include a minimal reproduction (code snippet, target triple, toolchain
  version, feature flags).
- Clarify whether the bug affects `no_std`, `serde`, or `std` builds.
- Attach profiler output or benchmarks when reporting performance regressions.
- For security-sensitive reports, email **nick@berlette.com** instead of
  filing a public issue.

---

## Local development

```sh
git clone https://github.com/nberlette/moos
cd moos
rustup component add rustfmt clippy
```

Run the same checks that CI enforces:

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features --no-deps -D warnings
cargo test --workspace --all-features
```

Helpful tips:

- Disable default features when testing `no_std` compatibility:
  `cargo test --no-default-features`.
- The `serde` feature is enabled by default; gate any `serde`-specific code
  with `#[cfg(feature = "serde")]`.
- Prefer small, focused commits with descriptive messages (present tense,
  active voice).

---

## Pull request guidelines

- Keep PRs scoped to a single logical change; large refactors should be split.
- Document API changes in `README.md` (or module-level docs) and update examples
  when behavior changes.
- Include tests or benchmarks demonstrating the fix/feature whenever possible.
- Ensure CI is green; GitHub Actions will run formatting, linting, tests, and
  crate publishing on release tags.
- Be ready to clarify design decisions or follow-up feedback during review.

---

## Release process

Releases are cut from annotated tags (e.g., `v0.1.0`). Tagging automatically:

1. Runs the full test matrix.
2. Publishes the crate to crates.io (requires `CARGO_REGISTRY_TOKEN` secret).
3. Creates a GitHub release if one does not already exist for the tag.

If you are not a maintainer, you do not need to run `cargo publish`; simply note
in your PR if the change warrants a release.

---

## Need help?

Open a discussion thread or ping `@nberlette` on the issue you are investigating.
We are happy to mentor first-time contributors — just let us know where you are
stuck. Happy hacking!
