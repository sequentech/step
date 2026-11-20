<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# board-messages threat model

board-messages is what is left of the crate that once held the braid bulletin-board messages and the electoral-log messages. Its message code moved to `b3` (`../b3/src/messages/`) and `electoral-log` (`../electoral-log/src/messages/`). The only file left is `src/electoral_log/mod.rs`: module declarations for code that is no longer here, plus two ignored immudb integration tests. There is no `Cargo.toml` or crate root and no build compiles it, so it runs nowhere. It matters to an election only if someone builds it again or takes it for the live audit-log message code. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Live electoral-log and board message code** (`../electoral-log/src/messages/`, `../b3/src/messages/`). Integrity: fixes and reviews must target the live code, not this copy.
- **Build integrity of the images whose build context includes this directory**. Integrity: only reviewed code should be linked into them.
- **immudb instances the tests reach**. Availability.
- **Development immudb credentials** in the test code. Confidentiality only in principle: they are the public development defaults.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/electoral_log/mod.rs` module declarations | A developer who adds the directory to a build | operator | No crate manifest or root; not a workspace member or a dependency of any crate; it refers to code that no longer exists, so it cannot compile |
| Ignored tests in `src/electoral_log/mod.rs` | A developer running ignored tests, if the module were built | operator | `#[ignore]`, `#[serial]`; not compiled |
| Image builds whose context includes this directory | CI and developer image builds | operator | No crate in those builds depends on it |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| board-messages-T1 | Tampering | The leftover module is linked into a component again (new manifest, workspace member or path dependency) and brings message logic that has drifted from `electoral-log` and `b3` | No crate manifest or root; not a workspace member or a dependency of any crate; it refers to code that no longer exists, so any build fails | Mitigated |
| board-messages-T2 | Repudiation | A review or fix of electoral-log message code targets this copy instead of the live code | Only the test module remains; the live message code is in `../electoral-log` and `../b3`; the system threat model marks this package as not built. The directory still exists | Partial |
| board-messages-T3 | Information disclosure | Credentials are kept in source | The test code holds only the public immudb development defaults; deployments take immudb credentials from the environment | Accepted (public development defaults in test code that is not compiled) |
| board-messages-T4 | Denial of service | The ignored tests destroy data on the immudb instance they reach | `#[ignore]` and `#[serial]`; test-only database names; not compiled | Accepted (test-only, needs an explicit run against a development immudb) |
| board-messages-T5 | Tampering | Unbuilt source travels in images and build contexts | No build compiles or runs it | Accepted (no secrets beyond T3, nothing executes it) |

## Assumptions

- The live message code in `electoral-log` and `b3` carries the real controls, and their threat models cover it.
- Developers do not restore this directory as a crate or dependency without bringing it in line with `electoral-log` and `b3` and reviewing it again.
- Deployments set their own immudb credentials and do not keep the development defaults.
- Ignored tests run only against disposable development immudb instances.

## Review focus

- Whether anything still builds, copies or imports this directory.
- Whether reviews and fixes of electoral-log messages land in `electoral-log` and `b3` rather than here.
