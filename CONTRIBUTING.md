# Contribution Guideline

Scarb is actively developed and open for contributions!
Want to get started?
Grab any unassigned issue labeled with [`good-first-issue`](https://github.com/orgs/software-mansion/projects/4/views/9)!
Need some guidance?
Reach out to other developers on [Telegram](https://t.me/+G_YxIv-XTFlhNWU0) or open a [discussion](https://github.com/software-mansion/scarb/discussions) on GitHub!

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
For larger for, try to make your commits small, self-contained and well described.
Each commit should pass lints and tests.
We are using rebase for merging pull requests, and thus we do not allow merge commits.

While your PR is being reviewed on, you can push merge commits and use [`git commit --fixup`](https://git-scm.com/docs/git-commit/2.32.0#Documentation/git-commit.txt---fixupamendrewordltcommitgt) to push further changes to your commits.
Then, when your PR will be accepted, you can autosquash your fixups with [`git rebase --autosquash`](https://git-scm.com/docs/git-rebase#Documentation/git-rebase.txt---autosquash).

---

Thanks! ❤️ ❤️ ❤️

Scarb Team
