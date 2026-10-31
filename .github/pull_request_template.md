## What

<!-- One or two sentences: what changes and why. Link the issue: Closes #123 -->

## How

<!-- Key design points. If this touches an architectural decision, link/add the ADR. -->

## Checklist

- [ ] Conventional Commit title (`feat(orm): ...`)
- [ ] Tests added/updated (unit, integration, `trybuild` for macros, `insta` snapshots reviewed)
- [ ] Docs updated (`///` on public items, `docs/` pages, CHANGELOG not needed — generated)
- [ ] `cargo fmt --all --check` and `cargo clippy --all-targets --all-features -D warnings` pass locally
- [ ] No new `unsafe`, no `anyhow` in library crates, no `unwrap()` outside tests
- [ ] Breaking change? → label `breaking` and describe the migration path below

## Migration notes (if breaking)

<!-- What users must change. -->
