# Scarb Agent Guidelines

## Stacked PRs

- This project uses stacked PRs managed with `git spr` or `git spice`. Before starting work, check whether the current branch is part of a stacked PR setup, for example with `git log --oneline main..HEAD` and local `spr` or `spice` config.
- If it is not clear whether we are working with stacked PRs, ask the user before making any commits.
- For large or multi-concern changes, ask the user whether they want to use a stacked PR approach before structuring commits.
- When stacked PRs are in use, amend changes into the appropriate commit rather than creating new ones, then use the relevant tool only when the user asks.

## Dependency Management

- For upgrading Cairo dependencies from the Cairo repo, use `cargo xtask upgrade cairo`.
- When upgrading dependencies, never use broad `cargo update`. Only update the specific packages requested using `--precise` and `--package` flags.
- Do not add or remove dependencies unless explicitly asked.
- Run `git diff main` after changes and verify only the intended dependencies changed. If unrelated deps appear, revert those specific changes.

## Testing

- Never run the full test suite without asking first. Run only targeted tests relevant to the changes, for example `cargo test -p <affected_crate>`.
