# Scarb Maintenance

## Maintainers

All Scarb maintainers are members of the **[@software-mansion/scarb-maintainers]** GitHub team.
Current project leader is **[@maciektr]**.

## Release procedure

Releasing Scarb is a semi-automated process.
It usually takes a couple of hours for me (~Marek), depending on if there are some last-minute patches to be made.
The longest part is writing release notes.

### Cut new version

In a nutshell, this is trivial: create a tag on `main` named `vX.Y.Z`.
There is a tag protection rule set up!
Make sure you create it on a green commit (CI is passing), this is not verified!
A tag should trigger a [Release] workflow which builds binaries, verifies them and drafts a GitHub release.
It usually takes about half an hour.

#### Create release branch

Sometimes `main` could be ahead with some commits that you might not want to include in this release.
It's totally fine to start a release branch in such scenarios.
The branch must be named `release/X.Y`, there's branch protection rule set up for this.
Then, cherry-pick commits that you might want to include.
After all is done, push the branch and a `vX.Y.Z` tag as usual!

As for website, don't forget to ensure that it should match latest Scarb version,
i.e. almost always be published from tags on `main`.
Release branches are allowed to be sources for website deployments,
and so they will deploy from tags in release branches.
You might need to swiftly pause the [Website Deploy] workflow to prevent publishing old website version.

> [!IMPORTANT]
> Make sure `scarb-metadata` crate is [released and published](#scarb-metadata-release-procedure) to [crates.io].
> Only [Starknet Crates.io Admins] can do this.

> [!IMPORTANT]
> When releasing against a new Cairo version (RC or final), follow the [Cairo version bumps](#cairo-version-bumps)
> procedure. Only release the downstream crates that actually need it.

> [!IMPORTANT]
> If you add or remove a binary target compiled with the release (i.e. create or remove a crate in `extensions` directory),
> make sure to work with the Homebrew team to upgrade their
> [Scarb installation formula](https://github.com/Homebrew/homebrew-core/blob/master/Formula/s/scarb.rb#L26).

### Write release notes

Only write release notes for stable releases, not release candidates.
Other releases should include auto-generated notes only.
For a stable release, compare the changes against the previous stable release in semver order,
not against the most recent release candidate.

Upon completion, the [Release] workflow should draft a release on GitHub.
Now comes the most tedious and time-consuming part that nobody likes: writing release notes!
We take an inspiration from the awesome release notes that [Visual Studio Code][vscode-relnotes] does.

1. Open the draft release.
2. Click `Generate release notes`, we will use this information in next step.
3. Leave release title intact.
4. If this is applicable, _✅ Set as a pre-release_.
5. If this is applicable, _✅ Set as the latest release_.
6. Use the **[template](#template)**.
7. Don't forget to _Save draft_ frequently!

#### Template

```markdown
[Cairo release notes ➡️](https://github.com/starkware-libs/cairo/releases/tag/vX.Y.Z)

Welcome to the release notes for Scarb vX.Y.Z!
This release is all about blahblah

* **Blah blah** - Why this blah is so much blah.
* **Even more blah blah** - Everything is awesome.
    * **Sometimes some sub-blah is cool** - Yeah.

## Highlight Point Title

Elaborate what this is about, what has changed, etc.
Be as descriptive as you can.

## Cairo Version

This version of Scarb comes with Cairo [`vX.Y.Z`](https://github.com/starkware-libs/cairo/releases/tag/vX.Y.Z).

## What's Changed

<!-- Here goes output from `Generate release notes` button. -->

**Full Changelog**: https://github.com/software-mansion/scarb/compare/...
```

> [!IMPORTANT]  
> The _Cairo version_ section is very important.
> Scarb website is parsing this sentence (the link to release notes, to be precise) to infer what version of Cairo is
> used in this release.

### Announce release on social media

#### Twitter

Post an announcement Tweet from your personal account, people tend to favour interacting with real humans accounts,
rather than corporate ones.

Here is example template, it's recommended to slightly change the copy, so that it won't look like robotic action:

```
Happy to announce that Scarb vX.Y.Z is live!

🔥 blahblah
🚀 even more blahblah

Go check it out: https://docs.swmansion.com/scarb/download.html
Release notes: https://github.com/software-mansion/scarb/releases/tag/vX.Y.Z

@swmansionxyz @StarkWareLtd
```

**Important:** The mentions are super important!
They seem to help generate more traffic.
Also post tweet link to `#crypto-mansion-marketing` and `#starkware-swm` Slack channels,
and do ensure that both Twitter accounts retweet your tweet on the same day.

#### Starknet Telegram

Post an announcement message on the _private Starknet channel where everybody is_ on Telegram.
All maintainers should be members of this group since their start.

Use similar text to one used in a Tweet, just attach Tweet link at the end, and do not mention anybody:

```
Announcing Scarb vX.Y.Z!

🔥 blahblah
🚀 even more blahblah

Download: https://docs.swmansion.com/scarb/download.html
Release notes: https://github.com/software-mansion/scarb/releases/tag/vX.Y.Z
Tweet: https://twitter.com/...
```

#### Starknet Discord

Post the same message as posted on the Telegram to the `#scarb` channel on Starknet's Discord.

## Cairo version bumps

Scarb bundles CairoLS and CairoLint, which form a dependency chain on top of Cairo:

```
cairo (cairo-lang-*)
  └─ cairo-language-common
       └─ cairo-lint
            └─ cairo-language-server (cairols)
                 └─ scarb
```

CairoLS also depends on Scarb crates (`cairo-lang-macro`, `scarb-proc-macro-host`, `scarb-proc-macro-server-types`,
`scarb-metadata`) and runs its tests against the Scarb binary from its `.tool-versions`.
Scarb also depends on `cairo-program-runner-lib` from the `proving-utils` fork, which requires `cairo-lang-*` too.

### General rule: don't bump Cairo downstream unless needed

Everything starts with Scarb. On a new stable Cairo version (minor or patch), bump `cairo-lang-*` **in Scarb only**.

Requirements like `cairo-lang-* = "2.21.0"` mean `^2.21.0` (`>=2.21.0, <3.0.0`), so Cargo resolves the whole graph
to the newer 2.x version bumped in Scarb.
If Scarb compiles with the current `cairo-language-common`, `cairo-lint` and `cairols` releases,
**do not bump Cairo in them and do not release them**.
Their versions may lag behind the Cairo version (e.g. Scarb with Cairo 2.22 and CairoLS 2.21), and that's fine.

Caret requirements do not match pre-releases, so a Cairo RC needs a release of the whole chain.
For the final Cairo version, re-release the chain to drop the `-rc` suffix.
Usually it's a version-only bump, unless the code changed between the RC and the final.

Bump Cairo in a downstream crate only when:

- the new Cairo changes an API that crate uses (Scarb doesn't compile), or
- the crate needs a new Cairo feature (e.g. new syntax support in CairoLS or CairoLint), or
- the new Cairo fails the crate's tests (e.g. corelib diagnostics or snapshots).

Then bump from the bottom of the chain, releasing only the crates that need it:
`cairo-language-common` -> `cairo-lint` -> `cairols` -> `scarb`.

The same applies to `scarb-proc-macro-host`, see [its release procedure](#scarb-proc-macro-host-release-procedure).

### Releasing a chain of crates

When downstream crates need a release (always the whole chain for a Cairo RC and the final that follows it):

1. Do not tag or publish immediately after preparing the branch.
   Push a same-named release branch for each repo that needs a release, open a PR, and wait for CI to go green.
2. For an RC, also release `scarb-proc-macro-host` (and `scarb-proc-macro-server-types` if it changed)
   before CairoLS.
3. Before tagging Scarb on a new Cairo version (RC or final), push a same-named `proving-utils` fork branch
   and pin Scarb to its commit, otherwise version solving can become unresolvable.
4. In chain order, publish each crate and push its `vX.Y.Z` tag.
   When a published crate is needed by the next repo, wait until it is visible on [crates.io] before continuing.

If you want agent assistance for this flow, use the `cairo-rc-release` skill from
`.agents/skills/cairo-rc-release/`.

### Scenarios

#### Cairo is released without affecting Scarb or CairoLS

1. Cairo is tagged on `main` and published to [crates.io].
2. Bump Cairo in Scarb. If everything compiles, you're done. CairoLS and CairoLint stay as they are.

#### Cairo is released and breaks an API used by CairoLint or CairoLS

1. Bump Cairo and fix the code only in the crates that don't compile, starting from the bottom of the chain.
2. Release them in chain order (see [Releasing a chain of crates](#releasing-a-chain-of-crates)).
3. Bump Cairo and the released crates in Scarb.

#### Scarb changes an interface CairoLS uses

Preferred: use a `dev-*` Scarb build.

1. Release a dev build of Scarb without CairoLS (the [Dev Build] workflow with `include-cairols: false`).
2. Push a commit to CairoLS `main` that uses this dev build (`.tool-versions`) and the new API
   (Scarb crates from git `rev`, or a published pre-release if one exists).
   If the Scarb change is updated before it merges, cut a new dev build and bump to it.
3. On the next Scarb release, bump CairoLS `main` to it and replace git revs with published Scarb crates.

Alternative, without a dev build (last resort only):

1. Push a commit to CairoLS `main` that uses the new interface, with Scarb crates patched by commit hash.
2. Ignore CairoLS tests that fail because `.tool-versions` still points to the old Scarb.
3. Release a Scarb nightly and publish the new crates to [crates.io].
4. Push a commit to CairoLS `main` that uses the new releases.

#### Cairo is released and breaks corelib

CairoLS has tests that check corelib produces no diagnostics.

Preferred: use a `dev-*` Scarb build.

1. Release a dev build of Scarb without CairoLS (the [Dev Build] workflow with `include-cairols: false`).
2. Push a commit to CairoLS `main` that uses this dev build (`.tool-versions`).
   If the Cairo bump in Scarb is updated before it merges, cut a new dev build and bump to it.
3. On the next Scarb release, bump CairoLS `main` to it.

Alternative, without a dev build (last resort only): mark the failing CairoLS tests `#[ignore] // wrong corelib version`,
bump `.tool-versions` to the next Scarb nightly, then remove the ignores.

#### Cairo is released from an old release branch while CairoLS `main` has moved on

CairoLS and CairoLint are released for every Cairo minor, so there is always a matching release to build on.

1. Bump Cairo in the Scarb release branch. Usually that's enough
   (e.g. Scarb 2.19.2-2.19.6 shipped CairoLS 2.19.1).
2. If a downstream crate doesn't compile, cut its patch release branch from its release for that Cairo minor,
   make only the change needed for the new Cairo (don't cherry-pick from `main`), and release it in chain order.

## `scarb-metadata` release procedure

Releasing `scarb-metadata` crate is also a semi-automated process.

1. Make sure correct version is present in:
    1. `scarb-metadata/Cargo.toml`
    2. `Cargo.lock`

   You will probably have to commit changes and then tag newly created commit.
2. Make sure you create it on a green commit (CI is passing), this is not verified!
3. Run `cargo publish -p scarb-metadata --dry-run` to verify that everything is fine.
4. Create a tag on `main` named `scarb-metadata/vX.Y.Z`.
5. Push it.
6. Run `cargo publish -p scarb-metadata`.

## `scarb-ui` release procedure

Releasing `scarb-ui` crate is also a semi-automated process.

1. Make sure correct version is present in:
   1. `scarb-ui/Cargo.toml`
   2. `Cargo.lock`

   You will probably have to commit changes and then tag newly created commit.
2. Make sure you create it on a green commit (CI is passing), this is not verified!
3. Run `cargo publish -p scarb-ui --dry-run` to verify that everything is fine.
4. Create a tag on `main` named `scarb-ui/vX.Y.Z`.
5. Push it.
6. Run `cargo publish -p scarb-ui`.

## `cairo-lang-macro` release procedure

Releasing `cairo-lang-macro` crate is also a semi-automated process.

1. Make sure correct version is present in:
    1. `plugins/cairo-lang-macro/Cargo.toml`
    2. `Cargo.lock`
       You will probably have to commit changes and then tag newly created commit.
2. Make sure all changes to crates `cairo-lang-macro-attributes`, `cairo-lang-macro-stable`, `scarb-stable-hasher` are
   released to crates.io as well.
3. Make sure you create it on a green commit (CI is passing), this is not verified!
4. Run `cargo publish -p cairo-lang-macro --dry-run` to verify that everything is fine.
5. Create a tag on `main` named `cairo-lang-macro/vX.Y.Z`.
6. Push it.
7. Run `cargo publish -p cairo-lang-macro`.

[@software-mansion/scarb-maintainers]: https://github.com/orgs/software-mansion/teams/scarb-maintainers

[@maciektr]: https://github.com/maciektr

[website deploy]: https://github.com/software-mansion/scarb/actions/workflows/website-deploy.yml

[release]: https://github.com/software-mansion/scarb/actions/workflows/release.yml

[dev build]: https://github.com/software-mansion/scarb/actions/workflows/build-dev.yml

[vscode-relnotes]: https://code.visualstudio.com/updates

[scarb project]: https://github.com/orgs/software-mansion/projects/4

[crates.io]: https://crates.io

[Starknet Crates.io Admins]: https://github.com/orgs/software-mansion/teams/starknet-crates-io-admins
