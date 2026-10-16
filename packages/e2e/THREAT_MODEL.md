<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# e2e threat model

e2e is test tooling with two binaries, run by developers and operators, never by voters. `e2e` (`src/main.rs`) fills the browser scenario templates in `src/scenarios/` with a target portal, an election event and test credentials, then creates, updates and launches the matching test on the third-party Loadero service (`src/services/loadero_service.rs`), whose browsers enrol, log in, vote or generate reports on that portal. `mock_server` (`src/mock_server/`) is a Rocket test double for an identity-verification provider plus a SQLite fixture store, started from an opt-in compose profile. No CI job builds an image of it. It matters to an election through the portals and election events it is pointed at and the credentials it holds. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Loadero API key**: confidentiality; it creates and launches browser load from the project's Loadero account.
- **Test credentials and target environments**: the test logins the scenarios use and the election events and portals they drive. Confidentiality of the credentials; integrity of the targeted ballots and reports; availability of the portals under load.
- **Mock identity dataset**: names, dates of birth and ID-card numbers held in the `mock_server` SQLite store. Confidentiality.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `e2e` CLI (`src/main.rs`), environment (Loadero URL and API key, polling interval, tenant ID) and scenario files (`src/scenarios/`) | Local user, repository contributors | operator | clap parsing; typed deserialization of scenario data |
| Loadero REST responses (`src/services/loadero_service.rs`) | Loadero service | untrusted | reqwest default TLS certificate verification; checked field access on `serde_json::Value` |
| `mock_server` HTTP routes (`src/mock_server/src/main.rs`, `src/mock_server/src/routes/`) | Anyone who reaches the container port | untrusted | Network isolation (deployment); parameterized SQL (`src/mock_server/src/services/user.rs`) |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| e2e-T1 | Information disclosure | The Loadero API key leaks | Sent only in the request `Authorization` header (`src/services/loadero_service.rs`); reqwest keeps TLS verification on | Partial |
| e2e-T2 | Spoofing | Test credentials the scenarios use leak and are used to sign in to a portal | Deployment (credential policy of the target environments) | Not verified |
| e2e-T3 | Tampering | A scenario run against a live election event casts synthetic ballots, triggers reports or loads production portals during voting | Operator procedure (deployment) | Not verified |
| e2e-T4 | Elevation of privilege | Test-only settings stay enabled on a realm that serves a real election | Realm configuration (deployment) | Not verified |
| e2e-T5 | Spoofing | A service configured to use `mock_server` as its identity-verification provider accepts every enrollment | Image build commented out in `../../.github/workflows/reusable_build_push.yml`; compose service only in an opt-in profile | Accepted (test double that always reports success; production must point at the real provider) |
| e2e-T6 | Information disclosure | Someone reads the identity records held by `mock_server` | Network isolation (deployment); parameterized SQL in `src/mock_server/src/services/user.rs` | Partial |

## Assumptions

- **Keycloak** realms that serve real elections use production configuration.
- **Operators** point scenarios only at test tenants and election events and keep the Loadero API key secret; **Loadero** (third party) protects the scripts and run data it stores.
- **Deployment**: `mock_server` runs only on isolated test networks with synthetic data and is never configured as a real identity-verification provider.

## Review focus

1. Secrets and test credentials used by the scenarios.
2. How scenario target environments and election events are chosen.
3. `mock_server` deployment and its container image.
