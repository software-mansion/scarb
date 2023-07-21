<script setup>
import { data as rel } from "../../github.data";
</script>

# Using Scarb in CI

To use Scarb in your CI workflow, you need to download the Scarb binary, unpack the archive, and add the directory
containing Scarb binary to your PATH variable.

## GitHub Actions

The officially supported [`software-mansion/setup-scarb`](https://github.com/software-mansion/setup-scarb) GitHub action
installs Scarb on the job runner and prepares environment for optimal use with dependency caching out of the box.
You can find an example of the Scarb setup in the following workflow file:

```yaml-vue
name: CI
on:
  push:
  merge_group:
  pull_request:
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: software-mansion/setup-scarb@v1
        with:
          scarb-version: "{{ rel.sampleVersion }}"
      - run: scarb fmt --check
      - run: scarb test
```

You can use `scarb-version` to specify which Scarb version will be used.
When it is not present, the action will resolve the version from `.tool-versions` file that's created when using [`asdf`](https://asdf-vm.com/guide/introduction.html).
In case there is no such file, the latest Scarb version will be installed.
You can find more information in action's repository.

<BigLink href="https://github.com/software-mansion/setup-scarb">
    Go to setup-scarb repository on GitHub
</BigLink>

## GitLab CI

You can find an example of the Scarb setup in the following GitLab CI configuration.

```yaml-vue
variables:
  SCARB_VERSION: "{{ rel.sampleVersion }}"

stages:
  - check

scarb:
  stage: check
  image: ubuntu:jammy
  script:
    - apt-get update && apt-get install -y curl
    - export PATH="$HOME/.local/bin:$PATH" && curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | bash -s -- -v $SCARB_VERSION
    - scarb fmt --check
    - scarb build
```

## CircleCI

You can find an example of the Scarb setup in the following workflow file.

```yaml-vue
version: 2.1

parameters:
  scarb_version:
    type: string
    default: "{{ rel.sampleVersion }}"

jobs:
  check:
    docker:
      - image: cimg/base:2023.03
    steps:
      - checkout
      - run:
          name: Setup Scarb
          command: |
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$BASH_ENV"
            source "$BASH_ENV"
            curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | bash -s -- -v << pipeline.parameters.scarb_version >>
      - run: scarb fmt --check
      - run: scarb build

workflows:
  ci:
    jobs:
      - check
```
