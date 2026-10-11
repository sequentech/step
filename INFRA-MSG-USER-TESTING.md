<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# INFRA-MSG prepared user testing

Parent issue: https://github.com/sequentech/meta/issues/13755

Prepared and activated 2026-10-10 for `/workspaces/step`, branch
`feat/meta-13755-msg-integration/ovcs`. Code fixes: `7627b1d3c9`, `31ad9a5302`, `86dabdc370`;
regenerated GraphQL contracts: `1e4c23dfbc`.
This packet supplements [the handover](INFRA-MSG-HANDOVER.md).

## Ready scenarios

Both events have eight synthetic voters, published ballots and online voting open.
They are ready for the user acceptance matrix below; do not create replacement events.

| Organization       | Voting                                                                                                                            | Admin event                                                                                             |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| First organization | [Open voting](http://localhost:3000/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/770e64cf-7820-47a2-85bd-368c39425e8a/login) | [Open admin](http://localhost:3002/sequent_backend_election_event/770e64cf-7820-47a2-85bd-368c39425e8a) |
| Staff Association  | [Open voting](http://localhost:3000/tenant/a0d6d85b-930e-4641-adcb-df6fa3ff60e8/event/2189d11a-5b03-4982-b10f-5f5e632ffabf/login) | [Open admin](http://localhost:3002/sequent_backend_election_event/2189d11a-5b03-4982-b10f-5f5e632ffabf) |

Development admin credentials are `admin` / `admin`, followed by the configured
email code. The scenario runner enrolled the second organization's administrator.
Use the admin portal tenant selector for Staff Association before opening its event.

## Readiness observed

| Check                | Result                                                                                                                                                                       |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Devcontainer         | Original container from October 7 remains running, never recreated or restarted                                                                                              |
| Backend              | Keycloak, Hasura, Harvest and relay healthy; Windmill and beat running                                                                                                       |
| Keycloak             | Current image rebuilt before activation; deployed message-OTP bytecode matches local source                                                                                  |
| Migration / metadata | All migrations present, including `1791200000000_messaging_second_pass`; all nine new viewer/insight actions present                                                         |
| Fake provider        | Enabled; Meta, Infobip and AWS endpoint overrides point to Harvest's fake routes                                                                                             |
| Service activation   | Beat and messaging-webhooks recreated by name; other backend services were already refreshed                                                                                 |
| GraphQL              | All four portals regenerated successfully; shared and CLI schemas refreshed separately                                                                                       |
| Portals              | Existing admin server preserved; missing voting server started; both return HTTP 200                                                                                         |
| Scenarios            | Both created successfully with key ceremonies, published ballots and online voting open                                                                                      |
| Checks               | 147 scenario/organization/monitoring tests; four portal type checks; focused results publication test, lint/formatting and REUSE pass; R10 review has no actionable findings |
| Provider smoke       | WhatsApp/Viber/SMS/email accepted; signed WhatsApp/Viber reports through relay answered HTTP 200; SMS/email correctly have no receipts                                       |
| Messenger refusal    | Account test to an unconnected recipient refused with `OUTSIDE_CONVERSATION_WINDOW`; Get Started/user verification still needs the matrix below                              |

Browser smoke: a synthetic voter in each organization can sign in, load its
published ballot and see **Change how we contact you**; the second has desktop
and mobile screenshots. The in-container browser
routes host-facing localhost service URLs to the same running internal services;
no API responses are mocked. The fake Messenger tester now reuses the saved
conversation for later sign-in links; all 14 fake-provider tests, Harvest
all-target clippy, Rust formatting and REUSE pass. Full channel journeys remain
user acceptance.

The public development callback URL is HTTP, so account checks warn **Callback
address must be https**. Fake reports use the internal relay and were verified;
real provider activation requires a reachable HTTPS callback URL. Synthetic
account-test messages for `+15550999001` (Viber normalizes away the plus) and
`activation-smoke@fake-provider.example.org` are visible in Sent messages;
exclude these from counts for the eight-voter bulk test. The unconnected
Messenger test attempts are deliberate failures, not failed scenario setup.

No further backend rebuild/restart is required for these fake-provider tests.
Fresh enrollment/manual review, Messenger Page changes, full browser journeys,
real Meta credentials and OTP-under-bulk latency remain acceptance work.

## Resume or recreate the scenarios (reference)

The scenarios already exist. These commands resume their recorded state if
needed. Use devenv: the system Python 3.10 cannot run the scenario CLI.

```sh
cd /workspaces/step
devenv shell -- scripts/dev/step-dev scenario up messaging --timeout 120
devenv shell -- scripts/dev/step-dev scenario up messaging-second-organization --timeout 120
```

Each command creates its own recorded election event, imports eight synthetic
voters, configures fake-provider accounts, completes the key ceremony, publishes
ballots and opens online voting. The commands may start the scenario's trustee
services. They resume interrupted setup instead of creating a duplicate event.
The second scenario creates/uses the `staff-association` tenant.

Get event URLs, voter usernames and development admin credentials from the
command output, or retrieve them again:

```sh
devenv shell -- scripts/dev/step-dev scenario urls messaging
devenv shell -- scripts/dev/step-dev scenario urls messaging-second-organization
devenv shell -- scripts/dev/step-dev scenario status messaging
devenv shell -- scripts/dev/step-dev scenario status messaging-second-organization
```

Every imported voter's synthetic password is `E2e-voter-2026!`. Usernames are
`e2e-messaging-a1` through `a4`, `b1`/`b2`, `c1`/`c2`; the second scenario
uses the same suffixes with `e2e-messaging-second-organization-`.
Imported voters start with verified synthetic email addresses but no explicit
messaging channel. Their initial password login is the bootstrap session;
the conditional channel code step starts after **Change how we contact you**
stores a verified channel. Verify Email there for b1 and the three control
voters as well, using their imported addresses. Enrollment/contact verification
is a separate journey, not proved by successful scenario creation.

## First scenario: successful paths

Open its admin and voting URLs from `scenario urls`. Keep an admin session
with **Settings > Messaging > Sent messages** open. Fake messages appear there;
no real messaging app or real phone number is needed.

Use these voters to prepare one contact per channel through the voting
portal's **Change how we contact you** flow:

| Voter suffix | Channel   | Synthetic contact                                        |
| ------------ | --------- | -------------------------------------------------------- |
| a1           | WhatsApp  | +15550101001                                             |
| a2           | Viber     | +15550101002                                             |
| a3           | Messenger | Account bound by Get Started on the session's m.me link  |
| a4           | SMS       | +15550101004                                             |
| b1           | Email     | Verify the imported synthetic address in the change flow |

Verify each change with the code in its Sent messages channel view. Messenger:
paste the connect page's m.me link in Sent messages > Messenger > Get Started;
then enter the received code in the original voter session. A successful change
replaces the contact, rather than adding another verified channel. The remaining
three voters use their verified Email channel and serve as controls for bulk counts.

If contact change is refused, capture its reason and check enabled reachable
code-step configuration: CHOSEN or NONE. Do not bypass the runtime gate.

| Test                    | Action                                                                              | Expected result                                                                                                         |
| ----------------------- | ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Settings and event save | Open Messaging settings and save the scenario event configuration                   | Accounts/status and event channels load; save succeeds; reloading preserves it                                          |
| One-channel sign-in     | Sign out and sign in as each prepared voter                                         | Code goes only to that voter's recorded channel; no alternate-channel choice or fallback                                |
| Resend/attempt limits   | Request another code within 60 s; then try five wrong codes in a fresh challenge    | Early resend waits; wrong attempts do not reset after resend; exhausted challenge requires restart                      |
| Contact replacement     | Change a1 from WhatsApp to Viber using another synthetic number                     | New contact saved only after verification; next sign-in uses Viber; restore WhatsApp for bulk test                      |
| Messenger reference     | Use the same Get Started reference again after successful verification              | No second code or second binding; used reference refused                                                                |
| Neutral reset           | Compare b1 with a nonexistent identifier; also test a1                              | Same neutral wording without channel/destination; no account-dependent error page; observe resend timer                 |
| Bulk notices            | Send one NOTICE to the eight scenario voters after restoring the channel allocation | Preview matches recipients; one message per voter on their channel; no fallback; counts agree in event, Logs and drawer |
| Delivery states         | Inspect WhatsApp, Viber, Messenger, SMS and Email results                           | App-channel reports reach DELIVERED through relay; SMS/email delivery receipts shown as unavailable, not invented       |
| Viewer                  | Switch All and all five channel views; refresh; copy an OTP; open an email          | Views and counts agree; copied code matches; email preview is sandboxed and its links inert                             |

For neutral reset, Messenger voters use staff-assisted recovery where the neutral
page cannot present a Messenger link. Neutral timing uses a 3 s floor; full
non-enumeration for slower provider calls is not claimed.

For fresh enrollment/manual review, use the scenario event's registration link
and configure the event's registration form to offer the channel under test,
with a deferred CHOSEN verification step. Do not change a NONE step silently:
NONE intentionally sends no code. Enroll new synthetic WhatsApp and Messenger
voters, checking contact attributes are stored only after code verification,
then pending-page channel/support reference and the approval/rejection notice.
Record this as additional setup and testing; the eight imported voters do not
prove fresh enrollment or manual review.

## Messenger Page-change regression

Use only the first scenario's fake Messenger account. Record its original Page
ID/username and configuration so you can restore them after testing.

1. Start a fresh Messenger contact change for a3 and tap Get Started. Keep the
   received code unverified in the voter tab.
2. Edit the fake account to another synthetic Page ID/username, re-enter its
   synthetic credentials when requested, and run its readiness check.
3. Submit the earlier code. The unfinished link must be EXPIRED; it must not
   save the old PSID against the new Page. Request a new link to continue.
4. Repeat the Page edit with a link whose Get Started has not yet been tapped.
   The old link must not bind/send on the replacement Page.
5. With an already verified old-Page voter, send a NOTICE: it must fail before
   provider dispatch with the translated **connected to another Page** reason.
6. Sign in after a validated password. The reconnect page must provide a new
   one-time link; Get Started and code verification bind the replacement Page.
   A reset flow must not offer the password-authenticated reconnect.
7. Restore the account and reconnect the test voter to the original Page before
   continuing other tests. Check no duplicate notice/code was sent by a late callback.

## Second tenant: expected restrictions

Repeat contact verification using that tenant's accounts and distinct contacts.
The prepared configuration intentionally differs:

| Channel   | Codes   | Notices |
| --------- | ------- | ------- |
| WhatsApp  | Allowed | Allowed |
| Viber     | Refused | Allowed |
| Messenger | Allowed | Refused |
| SMS       | Allowed | Refused |
| Email     | Allowed | Allowed |

The **Council: Canary Islands office** election allows WhatsApp/SMS/Email;
**Pension board** allows Email only. Verify preview and dispatch respect these
restrictions, refusals have translated reasons, and nothing falls back to another
channel. Account labels/organization copy must use Staff Association. Counts,
contacts, fake messages and support-reference searches must remain tenant scoped.
Viber cannot be verified for codes in this profile; a deliberately prepared
Viber notice recipient needs staff setup and is a separate restriction test.
Do not temporarily grant codes and count that as the original profile passing.

## Results to return

Run the core smoke test first: scenario creation, Messaging settings/event save,
WhatsApp contact verification and sign-in, Messenger Get Started and sign-in.
Then run the remaining matrix and Page-change cases on desktop and a narrow
mobile viewport. Record the scenario event IDs and final code revision.

| Case                                         | Pass / fail / blocked | Observed result / screenshot / safe error reason |
| -------------------------------------------- | --------------------- | ------------------------------------------------ |
| Scenario setup, both tenants                 |                       |                                                  |
| Settings and event save                      |                       |                                                  |
| WhatsApp verification and sign-in            |                       |                                                  |
| Messenger verification, replay and sign-in   |                       |                                                  |
| Other channels and contact replacement       |                       |                                                  |
| Neutral reset and attempt/resend limits      |                       |                                                  |
| Bulk counts and delivery states              |                       |                                                  |
| Page change before/after Get Started         |                       |                                                  |
| Password-authenticated reconnect             |                       |                                                  |
| Second-tenant restrictions/isolation         |                       |                                                  |
| Fresh enrollment/manual review (extra setup) |                       |                                                  |
| Desktop/mobile rendering                     |                       |                                                  |

Live Meta appsecret_proof and measured OTP-under-bulk latency remain separate
acceptance items in the handover. They are not established by the fake-provider
manual tests or the local passing suites.

## Cleanup

Keep fixtures until results are reviewed. When finished, these commands remove
only the recorded scenario events and their owned data:

```sh
devenv shell -- scripts/dev/step-dev scenario reset messaging
devenv shell -- scripts/dev/step-dev scenario reset messaging-second-organization
```

The fake accounts, second tenant and organization administrator intentionally
remain for reuse. Reset does not delete those shared scenario resources, stop
services, or alter unrelated events. Restore any Page/configuration edits first.

## Local provider configuration verified (2026-10-11)

Both prepared organizations use synthetic provider credentials. Harvest,
Windmill and beat override the Meta, Infobip and AWS endpoints with Harvest's
`/fake-provider/meta`, `/fake-provider/infobip` and `/fake-provider/aws` routes.
Provider callbacks enter the local relay at `http://messaging-webhooks:8410`.
The account check's HTTPS callback warning is expected for these internal HTTP
fixtures; successful signed delivery callbacks were verified separately.

All ten account checks passed. WhatsApp, Viber, SMS and email test sends were
accepted for each organization, using its existing permitted purpose (Staff
Association Viber remains notice-only). Messenger was tested through the
service-token link API and fake Get Started callback, reaching `CODE_SENT` and
then `CONFIRMED`; Meta/Infobip delivery reports returned HTTP 200. This establishes
local provider delivery and service authentication, not the remaining browser
acceptance cases above or delivery through real providers.

The main tenant's existing service-account user lacked `tenant-id`, so its token
was rejected by Harvest. The local user attribute was repaired, preserving
other attributes. Scenario setup now checks and repairs that attribute before
configuring messaging; a missing attribute also prevents reporting the messaging
stage complete. The checked-in realm already includes the correct attribute.

Smoke messages use `+15550999100`, `+15550999101` and
`demo-provider-check-{0,1}@fake-provider.example.org`; their text identifies the
local provider check. Exclude these synthetic messages from manual count checks.
