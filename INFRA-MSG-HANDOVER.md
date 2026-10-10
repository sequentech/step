<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# INFRA-MSG handover

Parent issue: https://github.com/sequentech/meta/issues/13755

Updated 2026-10-10. Work is in `/workspaces/step` on
`feat/meta-13755-msg-integration/ovcs`, based on `ovcs` at `8780ca92`.
Code checkpoint `7627b1d3c9` contains the final fix round; `31ad9a5302`
fixes activation prerequisites; `86dabdc370` fixes fake Messenger identity
reuse. Generated GraphQL contracts are committed separately (`1e4c23dfbc`). Nothing was pushed or rebased. The user subsequently authorized
stack activation; the devcontainer itself was never recreated or restarted.

## Prepared user testing continuation

[INFRA-MSG-USER-TESTING.md](INFRA-MSG-USER-TESTING.md) contains verified setup
commands, live event links, fixture identities, both tenants' expected channel
behavior, the Page-change regression and a results worksheet. Both synthetic
scenarios now have ballots published and online voting open. The current
Keycloak image matches the local message-OTP authenticator bytecode. All
migrations and messaging metadata were already present; they were verified,
not unnecessarily reapplied. Fake routes and provider overrides are enabled.
Beat and the callback relay were recreated by name; existing Harvest, Windmill
and Keycloak were already refreshed. Portal servers serve ports 3000 and 3002.

## Scope and current boundary

The second-pass messaging implementation and fix round 3 are complete on the
code side. The later instruction **"Do these, and make sure everything is
ready"** authorized activation of the named services and test scenarios.
Manual journey acceptance, OTP load latency, real Meta checks and local CI
remain pending. W4 (local CI and PR splitting) still follows user review of W3.

Activation fixed two reproduced setup failures: results queries used String
where the live schema requires UUID; scenario realm construction dropped the
kiosk client and copied foreign authentication-flow bindings when restoring it.
Regression tests ran red first; the final scenario suite passed 147 tests.
All four portal type checks, the focused results publication test, query lint
and formatting, and REUSE passed. Independent R10 review found no actionable
issues. Synthetic WhatsApp/Viber/SMS/email test sends succeeded; signed
WhatsApp/Viber reports reached Harvest through the relay with HTTP 200.
Both organizations' real browser voter login and published ballot loading
passed, with the contact-change entry visible and second-organization desktop
and mobile screenshots captured. Messenger correctly refused an
unconnected account-test recipient. SMS/email report
delivery receipts as unavailable. These are readiness checks, not proof of
the complete manual acceptance matrix.

The final review also found that fake Messenger Get Started assigned a new
identity to each one-time link, preventing subsequent sign-in. The fake route
now reuses the saved recipient from this tenant/account's pending, unexpired
link; new enrollment and reconnect retain distinct synthetic identities.
Nothing is returned from the encrypted held payload. The regression first
observed EXPIRED instead of CODE_SENT and now covers verification followed by
a new sign-in reference, actual code delivery to the saved recipient and
confirmation. The running Harvest watcher rebuilds this source automatically.
All 14 fake-provider tests passed, including the later sign-in confirmation;
Harvest all-target clippy, workspace Rust formatting and REUSE passed. R10
cleared the source fix and required Rust target-directory export.

User changes to the Keycloak image build context and concurrent admin messaging
layout/story edits remain unstaged and intact. They were not folded into the
activation commits. `devenv.lock` remains unchanged.

The implementation provides one verified voter channel without fallback,
account readiness and event configuration, same-channel retries, reserved OTP
capacity, a public callback relay with verification in internal Harvest,
provider adapters, development fake-provider routes, a Sent messages viewer,
and messaging insights. Production and staff guides:

- [Architecture](docs/docusaurus/docs/07-developers/14-messaging/01-messaging-architecture.md)
- [Deployment](docs/docusaurus/docs/07-developers/14-messaging/02-messaging-deployment.md)
- [Development](docs/docusaurus/docs/07-developers/14-messaging/03-messaging-development.md)
- [Manager reference](docs/docusaurus/docs/02-election_managers/02-reference/13-messaging-channels.md)

`INFRA-MSG-PLAN.md` and `INFRA-MSG-PROGRESS.md` remain local untracked planning
files as previously required. Detailed local evidence is in the git-excluded
`.agents/work/infra-msg/` directory. This tracked handover carries the essential
state so a fresh checkout does not depend on those local files.

## Review follow-through

Independent R9 reviewed fix round 3 against `dccf6a49`: no Critical or High
findings, two Medium findings. Both are fixed and re-reviewed in the final commit:

1. Neutral reset provider exceptions previously returned HTTP 500 only for
   existing users. They now render the neutral code page, preserve the resend
   timer, clear unsent codes and pending Messenger state, and log a safe reason.
   Code-only SMS providers refuse one-time links before generating a token.
   The normal provider-error and unsupported-link regressions were run red
   before the corresponding production fixes.
2. Messenger Page edits expire PENDING and CODE_SENT links in the account
   update transaction, deleting encrypted payloads. Creation, binding and
   confirmation lock account before link; a late send completion cannot revive
   an expired link. The database regression was red before the fix and green
   afterward. Existing schema and encrypted-payload formats remain compatible.

The callback limits are now passed through all four Compose deployments:
`MESSAGING_WEBHOOKS_MAX_IN_FLIGHT` (default 16) and
`MESSAGING_CALLBACK_CONNECTIONS` (default 4). Tracked environment templates
expose them. The deployment regression failed in all eight service/deployment
cases first, then the airgap bundle suite passed all five tests.

## Validation

All results below are local code checks on the final fix round, not live-stack
acceptance. Detailed local logs are `.agents/work/infra-msg/log/t3-code-*`.

| Scope                      | Result                                                                                                         |
| -------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Messaging / webhook relay  | 89 / 15 tests passed                                                                                           |
| Windmill library           | 1,392 passed; 7 existing ignored tests                                                                         |
| Windmill PostgreSQL suites | Nine suites passed (89 tests total after the added regression); final dispatch/account rerun: 19/12 passed     |
| Harvest                    | 441 passed; 2 existing ignored tests                                                                           |
| Keycloak Java              | 418 message-OTP + 82 voter-enrollment + 77 theme + 6 custom-listener tests passed                              |
| Keycloak UI                | 274 unit/browser tests passed; types, lint, formatting passed                                                  |
| Voting portal              | 856 tests passed; types, lint, formatting passed                                                               |
| Admin portal               | 1,832 tests + 25 messaging browser stories passed; types, lint, formatting and 500/500 widget inventory passed |
| Deployment                 | Airgap-bundle suite: 5 passed; callback wiring regression was red first                                        |
| WASM                       | All four archives match current sources; fingerprint `3be9b6d03aad9678`                                        |
| Licensing                  | `reuse lint` passed                                                                                            |

Reproduce Rust checks from the repository root in devenv, using
`CARGO_TARGET_DIR=/workspaces/step/packages/rust-local-target`:

```sh
cd packages
export CARGO_TARGET_DIR="$PWD/rust-local-target"
cargo test -p messaging -p messaging-webhooks
cargo test -p windmill --lib
cargo test -p windmill --test postgres_messaging --test postgres_messaging_account_checks --test postgres_messaging_accounts --test postgres_messaging_config --test postgres_messaging_dispatch --test postgres_messaging_insights --test postgres_messaging_migration --test postgres_messaging_rate_bucket --test postgres_messaging_retry
DEFAULT_SQL_LIMIT=20 LOW_SQL_LIMIT=1000 cargo test -p harvest
cargo fmt --all --check
cargo clippy --all-targets -p messaging -p messaging-webhooks -p windmill -p harvest
```

The Harvest audit test expects SQL limits 20/1000, unlike the development shell.
An initial invocation removed DEFAULT_SQL_LIMIT, causing configuration errors;
setting only that default exposed the low-limit mismatch. The final invocation
above passed the entire suite without source or runtime-config changes.

Java: from `packages/keycloak-extensions`, run
`mvn -q -pl message-otp-authenticator,voter-enrollment,sequent-theme,custom-event-listener -am test`.
Spotless passed for the initial four-module round and for the final changed
message-OTP module after its fixes. Frontend exact commands and local evidence
are in `reports/T3-code-jvmjs.md` under the local work directory.

The affected planner selects 84 checks conservatively because the existing
branch changes its selection model. `test --affected` and local CI were not
run: their wider CI-scale scope remains W4. Prettier flags all four Compose
files both at the prior HEAD and now; the two-line capacity changes retain
existing formatting. The new tracked handover is formatted.

The seven inactive scratch databases left by this continuation were removed
without forcing connections. Test/browser runners exited, and `devenv.lock`
is unchanged. The existing broken `beyond` submodule and local untracked
plan/progress files were preserved.

Final workspace `cargo fmt --all --check` and all-target clippy for messaging,
relay, Windmill and Harvest passed. Shared sequent-core type tests passed 60/60.
The final all-crate clippy run followed the Page-change fix.

## Stack activation reference (completed for this checkout)

No further backend rebuild/restart is needed for the prepared fake-provider
tests. The steps below remain a reference for another checkout. Public
callback readiness warns that the development HTTP address is not HTTPS;
real provider activation still requires a reachable HTTPS callback address.
Fake reports use the internal component URL and were verified separately.

Use the checkout's Compose project `step_devcontainer`, resolving host bind
paths from its host checkout (`/home/enes/step/.devcontainer` in this session).
Never recreate the devcontainer or run `compose down`.

1. Add these development-only values to the live `.devcontainer/.env` if absent:

   ```dotenv
   MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED
   MESSAGING_META_GRAPH_URL=http://harvest:8400/fake-provider/meta
   MESSAGING_INFOBIP_URL=http://harvest:8400/fake-provider/infobip
   MESSAGING_AWS_ENDPOINT_URL=http://harvest:8400/fake-provider/aws
   ```

   The live environment already had the relay URL, port and development relay
   secret in the previous handover. Verify them locally without publishing secrets.
   The new callback limits use their defaults unless overridden.

2. Apply migration `1791200000000_messaging_second_pass` and Hasura metadata.
   The migration intentionally deletes stored INBOUND records; rollback cannot
   restore them. Follow the deployment guide's rollback limitations.

   ```sh
   devenv shell -- bash -c 'cd hasura && hasura migrate apply --database-name backend-db --endpoint http://graphql-engine:8080 --admin-secret admin --skip-update-check'
   devenv shell -- bash -c 'cd hasura && hasura metadata apply --endpoint http://graphql-engine:8080 --admin-secret admin --skip-update-check'
   ```

3. Regenerate GraphQL schemas/types with `.devcontainer/scripts/update-graphql.sh`
   in devenv; it restarts graphql-engine. Review and commit generated changes
   separately. Restore the incidental `devenv.lock` rewrite after every shell.
4. Ensure `beyond/` exists as the empty uninitialized submodule directory, build
   the Keycloak service image, and recreate only Keycloak. Java providers and
   React themes come from the image; restarting the old image is insufficient.
5. Recreate `messaging-webhooks`, `harvest`, `windmill` and `beat` by name using
   `docker compose up -d --no-deps ...`. Recreating the relay picks up the new
   Compose concurrency variable. A restart retains old environment values.
   Beat runs `migrate_messaging_attributes` at startup. Until migration finishes,
   contact change can be refused by old BOTH code-step configurations.

## Remaining W3 acceptance

Use the already prepared scenarios and the user-testing worksheet:

- Confirm Settings > Messaging loads and event configuration saves.
- Run WhatsApp enrollment; read exactly one code in Sent messages; verify
  contact persistence and approval notice.
- Run Messenger enrollment through Get Started via the callback component,
  verify code and one-time reference refusal. Repeat with Page change during
  an unfinished link and with password-authenticated sign-in reconnect.
- Verify sign-in, contact replacement, neutral password reset, manual-review
  notice, bulk delivery and preview/count agreement on every configured channel.
- Both scenarios are already running; the second uses `staff-association`.
  Capture desktop/mobile screenshots and evidence during manual acceptance.
- Check the component's Meta handshake returns the challenge as plain text.
- Run `scripts/dev/step-dev bench messaging-otp` under saturated bulk load and
  retain measured samples; unit capacity tests do not establish live latency.
- On live Keycloak verify user PUT with `[]` removes an attribute, and
  `required-actions/{alias}/config` accepts `{"config":{"max_auth_age":"300"}}`.
- Verify `appsecret_proof` with real Meta credentials supplied by the user.
- Record stack ownership, health and cleanup; keep the devcontainer running.

Then obtain the planned W3 review before W4. W4 runs local CI and splits the
unpublished integration stack into the fewest PRs with at most 150 changed
files each, using `feat/meta-13755-msg-<n>-<slug>/ovcs`. PR descriptions start
with the parent-issue URL. No push or branch rearrangement happened here.

## Decisions and limitations to preserve

- Harvest stays internal. All public provider callbacks enter the relay.
- A configured NONE courier remains NONE and sends nothing; reachable enabled
  code steps otherwise migrate to CHOSEN. Orphan and disabled flows do not
  block contact change.
- Messenger reconnect requires a validated password plus the Messenger account
  that uses the session's one-time link; existing Messenger consent carries
  over. Forgotten-password recovery after a Page change requires staff.
- Neutral responses use a configurable 3000 ms floor (maximum 15000 ms).
  A send slower than that floor remains distinguishable by timing, and padding
  occupies a request thread. The HTTP/page failure oracle is fixed; complete
  timing non-enumeration is not claimed.
- Callback limits and replay protection are per process. Invalid Harvest callback
  capacity settings log an error and fall back; the relay rejects invalid values.
- Provider-number-change deduplication relies on the provider timestamp.
- No new external crates or npm packages were introduced in this continuation.
