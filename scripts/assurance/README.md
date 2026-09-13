<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Assurance lint policy

Production code must expose failures rather than suppress the type checker or
turn recoverable errors into panics. Unit and end-to-end tests may use `unwrap`,
`expect`, non-null assertions and similar fixture shortcuts. These checks
implement the first rollout of Sequent's Lightweight Assurance Methods. Passing this rollout does not mean all existing
code meets that policy or that a package has reached its coverage target.

| Language | Enforced scope | Check |
| --- | --- | --- |
| Rust | Sequent Core tally-sheet validation in non-test builds | The full agreed policy |
| TypeScript | UI Core production source | No explicit `any`, non-null assertions, TypeScript suppression comments, unsafe `finally`, or returned Promise executor values |
| Java | URL truststore provider production source | PMD: preserve causes, narrow exception catches, close resources, document contracts, and reject empty error handlers or direct console output |
| Python | Coverage tooling production source | Ruff `E`, `F`, `I`, `B`, `BLE`, `UP`, `RUF`, and `SIM` |

The TypeScript changes arrive in the UI Core PR above this tooling PR in the
stack. The commands below describe the integrated stack.

## Rust

From `packages/`:

```sh
cargo clippy --locked --no-deps --lib -p sequent-core --features default_features,keycloak
```

The shared Rust test setup action runs this command for its Sequent Core entry.
The tally-sheet validation module carries the policy for non-test builds. Normal
unit and integration tests keep their assertion style; `clippy.toml` explicitly
permits `unwrap`, `expect` and panic in tests. Do not mechanically replace a
production `unwrap` with `expect`: an infallibility claim needs a specific,
reviewable reason.

`packages/Cargo.toml` contains the canonical workspace policy. A retained crate
can inherit it with `[lints] workspace = true` after its existing violations are
fixed. Other crates, the remaining Core modules, and dependency code are not yet
clean; `--no-deps` limits the check to the selected crate. Existing work is tracked in
[Meta #11566](https://github.com/sequentech/meta/issues/11566).

## TypeScript

From `packages/`, run UI Core's `lint` command. The existing frontend lint
workflow invokes it too:

```sh
yarn --cwd ui-core lint
```

Use a runtime check to narrow uncertain production input. Unit tests, browser
tests and stories keep their existing checks and may use assertion shortcuts.
UI Essentials and Voting Portal still need their production lint rollout.

## Java

From the repository root:

```sh
mvn -B verify --file packages/keycloak-extensions/url-truststore-provider/pom.xml
```

The existing Java Test workflow also reaches this check through its aggregate
`verify` build. The module pins Maven PMD Plugin 3.28.0 and PMD 7.27.0, checks
production code, and fails on every reported priority. Tests are excluded from
these additional error-handling restrictions. Other Keycloak modules do not
inherit it automatically: the aggregate POM is not their parent.

The truststore's fallback catches its own loading exception, which retains the
original I/O or certificate cause. Unrelated programming errors must not be
mistaken for a temporary certificate-service outage. Invalid hostname policy
configuration also retains its cause.

The pinned engine override follows the
[Maven PMD instructions](https://maven.apache.org/plugins/maven-pmd-plugin/examples/upgrading-PMD-at-runtime.html).
Rules live in `java.xml`; do not suppress a finding just to obtain a passing build.

## Python

From the repository root, using the tool versions pinned in the coverage job:

```sh
ruff check --config scripts/coverage/ruff.toml scripts/coverage
ruff format --check --config scripts/coverage/ruff.toml scripts/coverage
python3 -m unittest discover -s scripts/coverage
```

The rules include broad-exception detection and checks for unsafe control flow.
Preserve useful failure context and keep resource cleanup explicit. Tests retain
the pre-existing syntax, undefined-name and import checks; the new production
rule families are exempted for `test_*.py`.
