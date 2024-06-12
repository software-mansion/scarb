# Contribution Guideline

Scarb is actively developed and open for contributions!

*Want to get started?*
Grab any unassigned issue labeled with [`help-wanted`](https://github.com/software-mansion/scarb/labels/help%20wanted)!

*Looking for some easy warmup tasks?*
Check out issues labeled with [`good-first-issue`](https://github.com/software-mansion/scarb/labels/good%20first%20issue)!

*Need some guidance?*
Reach out to other developers on [Telegram](https://t.me/+1pMLtrNj5NthZWJk)!

## Environment setup

Latest stable Rust is the only thing you really need.
It is recommended to use [rustup](https://rustup.rs/) for getting it.

If you wish to work on Scarb's website, you will need [Node.js](https://nodejs.org/).
We recommend to install it using [asdf](https://asdf-vm.com/) (via [nodejs](https://github.com/asdf-vm/asdf-nodejs) plugin).

## Contributing

Before you open a pull request, it is always a good idea to search the issues and verify if the feature you would like
to add hasn't been already discussed.
We also appreciate creating a feature request before making a contribution, so it can be discussed before you get to
work.

### Writing Tests

Please make sure the feature you are implementing is thoroughly tested with automatic tests.
You can check test already in the repository, to see how to approach that.

### Breaking Changes

If the change you are introducing is changing or breaking the behavior of any already existing features, make sure to
include that information in the pull request description.

### Running Tests and Checks

Before creating a contribution, make sure your code passes the following checks:

```shell
cargo fmt
cargo clippy
cargo test
```

Otherwise, it won't be possible to merge your contribution.

### Git

Try to make small PRs, that could be squashed into a single commit.
For larger work, try to make your commits small, self-contained and well described.
Each commit should pass lints and tests.
Then, set up a stack of pull requests, separate PR for each commit, and pointing to the previous one.

While your PR is being reviewed on, you can push merge commits and
use [`git commit --fixup`](https://git-scm.com/docs/git-commit/2.32.0#Documentation/git-commit.txt---fixupamendrewordltcommitgt)
to push further changes to your commits.

### Typos
Our policy is to not accept PRs that only fix typos in the documentation and code. We appreciate your effort, but we
encourage you to focus on bugs and features instead.

---

Thanks! ❤️ ❤️ ❤️

Scarb Team
