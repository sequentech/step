<!--
SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# new-ballot-verifier

[![Chat][discord-badge]][discord-link]
[![Build Status][build-badge]][build-link]
[![codecov][codecov-badge]][codecov-link]
[![Dependency status][dependencies-badge]][dependencies-link]
[![License][license-badge]][license-link]
[![REUSE][reuse-badge]][reuse-link]

Sequent cast-as-intended verifier. It allows a voter to audit an (spoiled) ballot. `ballot-verifier` implements the 'cast or cancel' procedure described on the paper [Ballot Casting Assurance via Voter-Initiated Poll Station Auditing](https://www.usenix.org/legacy/event/evt07/tech/full_papers/benaloh/benaloh.pdf) by Josh Benaloh.

# 1. Continuous Integration

There are multiple checks executed through the usage of Github Actions to verify
the health of the code when pushed:
1. **Compiler warning/errors**: checked using `cargo check` and 
`cargo check ---tests`. Use `cargo fix` and `cargo fix --tests` to fix the 
issues that appear.
2. **Unit tests**: check that all unit tests pass using `cargo test`.
3. **Code style**: check that the code style follows standard Rust format, using
`cargo fmt -- --check`. Fix it using `cargo fmt`.
4. **Code linting**: Lint that checks for common Rust mistakes using 
`cargo clippy`. You can try to fix automatically most of those mistakes using
`cargo clippy --fix -Z unstable-options`.
5. **Code coverage**: Detects code coverage with [cargo-tarpaulin] and pushes
the information (in master branch) to [codecov].
6. **License compliance**: Check using [REUSE] for license compliance within
the project, verifying that every file is REUSE-compliant and thus has a 
copyright notice header. Try fixing it with `reuse lint`.
7. **Dependencies scan**: Audit dependencies for security vulnerabilities in the
[RustSec Advisory Database], unmaintained dependencies, incompatible licenses
and banned packages using [cargo-deny]. Use `cargo deny --all-features check`
to try to solve the detected issues. We also have configured [dependabot] to
notify and create PRs on version updates.
8. **Benchmark performance**: Check benchmark performance and alert on
regressions using `cargo bench` and [github-action-benchmark].
9. **CLA compliance**: Check that all committers have signed the 
[Contributor License Agreement] using [CLA Assistant bot].
10. **Browser testing**: Check the library works on different browsers and operating
systems using [browserstack](https://www.browserstack.com/). Run `npm run local`
on the `browserstack` folder to try it locally. You'll need to configure the env variables 
`GIT_COMMIT_SHA`, `BROWSERSTACK_USERNAME`, `BROWSERSTACK_ACCESS_KEY`.

# 2. Development environment

new-ballot-verifier uses [Github dev containers] to facilitate development. To start developing 
new-ballot-verifier, clone the github repo locally, and open the folder in Visual Studio Code 
in a container. This will configure the same environment that new-ballot-verifier developers 
use, including installing required packages and VS Code plugins.

We've tested this dev container for Linux x86_64 and Mac Os arch64 architectures. Unfortunately
at the moment it doesn't work with Github Codespaces as nix doesn't work on Github Codespaces yet.
Also the current dev container configuration for new-ballot-verifier doesn't allow commiting 
to the git repo from the dev container, you should use git on a local terminal.

# 3. Nix reproducible builds

new-ballot-verifier uses [Nix Package Manager] as its package builder. To build
new-ballot-verifier, **first [install Nix]** correctly in your system.

After you have installed Nix, enter the development environment with:

```bash
nix develop
```

Note that if you're using dev containers with VS Code, this will already happen
automatically when you open a new console.

# 4. Compiling rust library

Assuming a starting point from the root folder of this repo and after executing
nix develop, run the following command to compile the rust code into a WASM target,
which you'll need to build the React app:

```bash
nix build -vv -L
```

The build should be pretty quick if you're using VS Code with Dev Containers as
it should be pre-built already.  This generates a tar file in a format that the
yarn package manager can use. Now you'll need to copy the npm package to the folder
where npm will import it from:

```bash
mkdir -p rust/pkg
cp result/new-ballot-verifier-lib-*.tgz rust/pkg/
```

We've compiled the rust library and copied it to the folder yarn will use. Now, if
the rust code has changed then the hash of the npm package will have changed as well
so you need to update the `yarn.lock` file with the new hash. You can do that with
these commands:

```bash
export HASH=$(sha1sum ./rust/pkg/new-ballot-verifier-lib-0.1.0.tgz | cut -d" " -f 1)
sed -i "s/new-ballot-verifier-lib-0\.1\.0\.tgz#.*/new-ballot-verifier-lib-0.1.0.tgz#${HASH}\"/g" yarn.lock
```

In a similar fashion, you'll also need a version of `sequent-core` compiled with the
WASM target and packaged in a tar file that can be imported by node/yarn. Clone the
github project, and open it in a new VS Code window as a Dev Container. You can build
the required package by running this command on the `sequent-core` Dev Container:

```bash
nix build -vv -L
```

This will generate a file at `result/sequent-core-0.1.0.tgz` in the `sequent-core` folder.
As `result` is a folder that can only be seen inside the `sequent-core` Dev Container,
you'll need to copy it. Run this inside the `sequent-core` Dev Container:

```bash
cp result/sequent-core-*.tgz .
```

Then run this in a terminal outside any Dev Container. Here we'll assume that you have
both github projects inside the same folder:

```bash
cp sequent-core/sequent-core-*.tgz new-ballot-verifier/rust/pkg
```

Finally you'll also need to update the `yarn.lock` file with the latest hash of the 
sequent-core package:

```bash
export HASH=$(sha1sum ./rust/pkg/sequent-core-0.1.0.tgz | cut -d" " -f 1)
sed -i "s/sequent-core-0\.1\.0\.\.tgz#.*/sequent-core-0.1.0.tgz#${HASH}\"/g" yarn.lock
```

# 5. Compile the Ui Core library

The ballot verifier uses the common UI librarry [ui-core] as a github submodule.
If you're using devcontainers it should already be checked in, otherwise run:

```bash
git submodule update --init
```

Then you need to compile ui-core:

```bash
cd ui-core
yarn
yarn build
```

# 6. Compile the Ui library

The ballot verifier uses the common UI librarry [ui-essentials] as a github submodule.
If you're using devcontainers it should already be checked in, otherwise run:

```bash
git submodule update --init
```

Then you need to compile ui-essentials:

```bash
cd ui-essentials
yarn
yarn build
```


# 6.5. Launching the UI in development mode

Once you have compiled the rust code and within the `nix develop` environment,
you can start the react development server from the repository's root folder:

```bash
yarn # install dependencies
yarn start
```

# 7. Production Build

Once you have compiled the rust code and within the `nix develop` environment,
you can build the project for production with `yarn build` in the repository's
root folder. The result will be available in the `build/` folder.

## Storybook

To run storybook, execute `yarn storybook`, then open `localhost:6006`. 


[cargo-deny]: https://github.com/EmbarkStudios/cargo-deny
[cargo-edit]: https://crates.io/crates/cargo-edit
[codecov]: https://codecov.io/
[REUSE]: https://reuse.software/
[cargo-tarpaulin]: https://github.com/xd009642/tarpaulin
[github-action-benchmark]: https://github.com/benchmark-action/github-action-benchmark

[Contributor License Agreement]: https://cla-assistant.io/sequentech/new-ballot-verifier?pullRequest=27
[CLA Assistant bot]: https://github.com/cla-assistant/cla-assistant
[dependabot]:https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuring-dependabot-version-updates
[RustSec Advisory Database]: https://github.com/RustSec/advisory-db/
[Nix Package Manager]: https://nixos.org/
[install Nix]: https://nixos.org/
[Github dev containers]: https://docs.github.com/en/codespaces/setting-up-your-project-for-codespaces/introduction-to-dev-containers

[discord-badge]: https://img.shields.io/discord/1006401206782001273?style=plastic
[discord-link]: https://discord.gg/WfvSTmcdY8

[build-badge]: https://github.com/sequentech/new-ballot-verifier/workflows/CI/badge.svg?branch=main&event=push
[build-link]: https://github.com/sequentech/new-ballot-verifier/actions?query=workflow%3ACI

[codecov-badge]: https://codecov.io/gh/sequentech/new-ballot-verifier/branch/main/graph/badge.svg?token=W5QNYDEJCX
[codecov-link]: https://codecov.io/gh/sequentech/new-ballot-verifier

[dependencies-badge]: https://deps.rs/repo/github/sequentech/new-ballot-verifier/status.svg
[dependencies-link]: https://deps.rs/repo/github/sequentech/new-ballot-verifier

[license-badge]: https://img.shields.io/github/license/sequentech/new-ballot-verifier?label=license
[license-link]: https://github.com/sequentech/new-ballot-verifier/blob/master/LICENSE

[reuse-badge]: https://api.reuse.software/badge/github.com/sequentech/new-ballot-verifier
[reuse-link]: https://api.reuse.software/info/github.com/sequentech/new-ballot-verifier

[ui-essentials]: https://github.com/sequentech/ui-essentials

# 8. Troubleshooting

## 8.1 Cargo.lock was not updated

This is a common error:

```bash
new-ballot-verifier-lib> Validating consistency between /tmp/nix-build-new-ballot-verifier-lib-0.0.1.drv-0/0bq2m6wf3nncdgz7296z5c3pzi773m4j-source//Cargo.lock and /tmp/nix-build-new-ballot-verifier-lib-0.0.1.drv-0/cargo-vendor-dir/Cargo.lock
new-ballot-verifier-lib> Finished cargoSetupPostPatchHook
new-ballot-verifier-lib> updateAutotoolsGnuConfigScriptsPhase
new-ballot-verifier-lib> configuring
new-ballot-verifier-lib> building
new-ballot-verifier-lib> PHASE Build: wasm-pack build
new-ballot-verifier-lib> Error: Error during execution of `cargo metadata`:     Updating git repository `https://github.com/sequentech/sequent-core`
new-ballot-verifier-lib> error: failed to write /tmp/nix-build-new-ballot-verifier-lib-0.0.1.drv-0/0bq2m6wf3nncdgz7296z5c3pzi773m4j-source/Cargo.lock
new-ballot-verifier-lib> Caused by:
new-ballot-verifier-lib>   failed to open: /tmp/nix-build-new-ballot-verifier-lib-0.0.1.drv-0/0bq2m6wf3nncdgz7296z5c3pzi773m4j-source/Cargo.lock
new-ballot-verifier-lib> Caused by:
new-ballot-verifier-lib>   Permission denied (os error 13)
error: builder for '/nix/store/8lg86x80v5ccq41f0xzwdbg8navqf2is-new-ballot-verifier-lib-0.0.1.drv' failed with exit code 1
```

This happens because you updated the dependencies in the `Cargo.toml` file but failed to update the dependencies in `Cargo.lock.copy`, which is the file used by nix. Go to `5. Updating dependencies` to build the library locally to obtain a new `Cargo.lock` file.