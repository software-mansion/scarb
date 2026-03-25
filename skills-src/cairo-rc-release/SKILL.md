---
name: cairo-rc-bump-release
description: Use when preparing a Cairo release candidate bump across local sibling repos such as cairo-language-common, cairo-lint, cairols, scarb, and proving-utils. Covers the patchless release flow, crates.io publication ordering, required proving-utils pinning for Scarb RC releases, same-named branch and tag conventions, and requires explicit user approval before every mutating action including edits, git operations, lockfile regeneration, pushes, tags, and cargo publish.
---

# Cairo RC Bump Release

Use this workflow for coordinated Cairo RC bumps across local sibling repos such as:

- `../proving-utils`
- `../cairo-language-common`
- `../cairo-lint`
- `../cairols`
- `../scarb`

## Prerequisites

- The target version is explicit, for example `2.17.0-rc.4`.
- The relevant sibling repos exist locally.
- Read each repo's `MAINTAINING.md` and any local agent instructions before changing anything.
- The user is available to approve every mutating action.
- Git remotes and crates.io auth are already configured for repos that must be pushed or published.
- Release manifests must end in a clean release state. Do not hand-edit `[patch.crates-io]`; use the repo's xtask upgrade flow and then verify there are no leftover unused patches.

## Approval Rule

Before every mutating action, tell the user exactly what will be done next and wait for approval.

Treat all of these as mutating:

- file edits
- lockfile regeneration
- `git fetch`
- worktree creation
- branch creation
- commit
- reset
- rebase
- push
- tag creation
- tag push
- `cargo publish`

Do not continue to the next mutating step until the user approves it.

## Branching Rule

- Never assume `main` is the release branch.
- Prefer a dedicated worktree or branch named exactly as the RC version, for example `2.17.0-rc.4`.
- Do not push `main` unless the user explicitly asks.
- If local `main` was used by mistake, move the work onto the release branch first, then reset local `main` back to `origin/main`.

## Checkout Verification Rule

- Before cutting any release branch or worktree, verify that the repo has been checked out from the latest `main` commit.
- For normal repos, confirm `origin/main` is up to date locally, inspect whether local `main` matches it, and base the release branch or worktree on that latest `origin/main`.
- If the repo is not on the latest `main`, stop and ask before doing any syncing operation such as fetch, pull, reset, or worktree creation.
- Do not guess that an existing local branch is current just because its name matches the release version.

## Required Release Order

1. `proving-utils`
2. `cairo-language-common`
3. `cairo-lint`
4. `cairo-language-server` / `cairols`
5. `scarb`

## Core Per-Repo Workflow

1. Inspect repo state first: branch, worktree, remotes, and dirtiness.
2. Verify that the release work will start from the latest `main` commit.
3. Ask permission to fetch and create or switch to the same-named release worktree or branch.
4. Use the repo's xtask upgrade commands for Cairo-related dependency bumps so the release state stays patchless.
5. Refresh `Cargo.lock` with normal Cargo resolution.
6. Run targeted verification, including checking for leftover unused patches and running a clean `cargo check`.
7. Ask before commit.
8. Ask before push.
9. Push the same-named branch and provide the PR URL.
10. Wait for the user to confirm CI passed on that PR.
11. Ask before tag creation and tag push, if that repo is tagged.
12. Ask before `cargo publish`, if that crate is supposed to be released now.

## Repo Rules

### `proving-utils`

- Treat this as a required prerequisite for the Scarb RC release.
- Verify both the upstream `main` tip and the fork remote state before starting release work there.
- Confirm the release branch is based on the latest upstream `main`.
- Also verify that the fork remote's `main` is synced to that upstream `main` commit. If it is not, stop and ask before doing any syncing or corrective push on the fork.
- Bump Cairo dependencies to the exact RC version in the fork checkout the user provides.
- Refresh `Cargo.lock`.
- Commit on the same-named branch and push that branch to the requested fork remote.
- Use the resulting commit SHA in Scarb so the workspace remains resolvable.

### `cairo-language-common`

- Bump package version to the RC.
- If the repo provides an xtask helper for Cairo upgrades, use `cargo xtask upgrade cairo <VERSION>`. Otherwise apply the equivalent manifest change manually.
- Set `cairo-lang-*` dependencies to the exact RC version.
- Do not hand-edit `[patch.crates-io]`.
- Verify with `cargo metadata --format-version 1` and a clean `cargo check`.
- Push the same-named release branch and create a PR before asking for any tag or publish step.

### `cairo-lint`

- Bump workspace version to the RC.
- Use `cargo xtask upgrade cairo <VERSION>`.
- Set `cairo-language-common` to the exact RC version after it is published.
- Do not hand-edit `[patch.crates-io]`.
- Verify with `cargo metadata --format-version 1` and a clean `cargo check`.
- Push the same-named release branch and create a PR before asking for any tag or publish step.

### `cairols`

- Bump package version to the RC.
- Use `cargo xtask upgrade cairo <VERSION>`.
- Set `cairo-language-common` and `cairo-lint` to the exact RC version after they are published.
- Do not hand-edit `[patch.crates-io]`.
- Verify with `cargo metadata --format-version 1` and a clean `cargo check`.
- Push the same-named release branch and create a PR first.
- If the repo is tagged, create `v<version>` only after the PR exists, CI is green, and the user approves.

### `scarb`

- Never do the release work on local `main`.
- Keep the work on the same-named RC branch.
- Set `workspace.package.version` to the RC.
- Use `cargo xtask upgrade cairo <VERSION>` to bump Cairo crates.
- Use `cargo xtask upgrade cairo-lint <VERSION>` to bump the lint dependency if that xtask exists in the repo.
- Set `cairo-language-server` and `cairo-lint` to the exact RC version only once those crates are visible on crates.io.
- Do not hand-edit `[patch.crates-io]`.
- Before finalizing Scarb, update `proving-utils` in the user-provided fork checkout, commit the Cairo RC bump there on the same-named branch, push that branch, and point Scarb to the exact commit SHA. This is required for Scarb RC releases because otherwise version solving can become unresolvable.
- Verify with:
  - `cargo metadata --format-version 1`
  - `cargo check -p scarb-execute`
- Push the same-named release branch so the user can open or update the release PR and wait for CI before any later release step.

## PR And CI Gate

- Every releasable repo should have a same-named release branch pushed first.
- Provide the PR URL as soon as the branch is pushed.
- Do not move to tag creation or publishing until the user confirms the PR checks passed.
- If the repo is only an intermediate dependency and does not need a tag, still use the PR as the CI gate before asking to publish.

## crates.io Propagation Rule

If downstream resolution fails because a newly published crate is not yet visible on crates.io:

- stop
- verify with `cargo info crate@version`
- wait until that command succeeds
- only then continue with the next repo

Do not replace the release dependency with a patch section just to keep moving.

## Publish Rule

- `cargo publish` is allowed, but only after an explicit approval message immediately before the command.
- Before asking to publish, confirm that the release PR exists and the user has confirmed CI passed on it.
- After publishing, poll with `cargo info crate@version` until the requested version is visible before continuing downstream.

## Upgrade Command Rule

- Prefer repo-provided upgrade helpers over hand-editing Cairo dependency graphs.
- For Cairo dependency bumps, use `cargo xtask upgrade cairo <VERSION>` in repos that provide it.
- For Scarb lint dependency bumps, use `cargo xtask upgrade cairo-lint <VERSION>` if that helper exists in the repo.
- After running an xtask helper, inspect the resulting manifest and lockfile to confirm the requested RC version was applied exactly.
- Do not clean up `[patch.crates-io]` by hand after xtask.
- Instead, verify there are no leftover unused patches. If unused patches remain, treat that as a bad sign and stop to investigate rather than silently editing them away.
- Also run a clean `cargo check` for the affected repo or targeted crate path before committing.
- Only fall back to manual dependency edits if the repo does not provide the required xtask command.

## Messaging Style

Keep updates short and concrete. Before each mutating step, say exactly what will happen next.

Examples:

- `Waiting for your permission to fetch origin in ../cairo-lint and verify the release branch will be based on the latest origin/main.`
- `Waiting for your permission to check whether the proving-utils fork remote main is synced with upstream main before creating branch 2.17.0-rc.4.`
- `Waiting for your permission to fetch origin in ../cairols and create worktree 2.17.0-rc.4 from origin/main.`
- `Waiting for your permission to commit the cairo-lint release changes in ../cairo-lint-2.17.0-rc.4.`
- `Waiting for your permission to push branch 2.17.0-rc.4 so you can open a PR and watch CI.`
- `CI is green on the release PR. Waiting for your permission to create and push tag v2.17.0-rc.4.`
- `Waiting for your permission to run cargo publish for cairo-language-server 2.17.0-rc.4.`
