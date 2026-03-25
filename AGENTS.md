# Scarb Agent Guidelines

## Git Rules

- Never commit or push code unless explicitly asked. Always show proposed changes and wait for approval before any git commit, git push, or stack management operations (`spr`, `spice`, etc.).
- After generating code, run lint and format like in CI. Fix any issues before proceeding.

## Stacked PRs

- This project uses stacked PRs managed with `git spr` or `git spice`. Before starting work, check whether the current branch is part of a stacked PR setup (e.g. run `git log --oneline main..HEAD` and check for multiple logical commits or a `spr`/`spice` config).
- If it is not clear whether we are working with stacked PRs, ask the user before making any commits.
- For large or multi-concern changes, ask the user whether they want to use a stacked PR approach before structuring commits.
- When stacked PRs are in use, amend changes into the appropriate commit rather than creating new ones, then use the relevant tool (`git spr update`, `gs` commands, etc.) to sync the stack — but only when the user asks.

## Dependency Management

- For upgrading cairo dependencies (from cairo repo specifically), use `cargo xtask upgrade cairo`
- When upgrading dependencies, never use broad `cargo update`. Only update the specific packages requested using `--precise` and `--package` flags.
- Do not add or remove dependencies unless explicitly asked.
- Run `git diff main` after changes and verify only the intended dependencies changed. If unrelated deps appear, revert those specific changes.

## Testing

- Never run the full test suite without asking first. Run only targeted tests relevant to the changes (e.g., `cargo test -p <affected_crate>`).

## Workflow

- This repo uses git worktrees. When starting work, confirm which worktree/branch to operate in. Run `git worktree list` first if there is any ambiguity.

## Rust Conventions

- When refactoring error types or enums, strictly respect the separation boundaries defined. Do not mix concerns (e.g., parse vs. semantic errors) even if it seems convenient.
