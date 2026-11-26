<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Offline fixture authority for Initialize signing

Use only with the disposable `operational_demo_14990` database, never the
hosted service or a receiver. The authority utility uses a canonical **local
Unix socket**, exact installed run/tenant/event/provider, and the fixture
database operator's separate login. It deliberately ignores ambient PostgreSQL
routing settings (including environment-supplied passwords). The operator
login needs to provision the fixture staff CA, CRL and roster and to register
certificates under the designated fixture Security Officer. Do not grant
these permissions to Harvest, browser clients, broker or device workers.
DB-side checks are independent of the CLI check.

1. Start the isolated device, broker, and Harvest with their pinned roles and
   the same sealed installation. Use three existing, distinct synthetic
   identities: two officials with the ordinary Initialize Signing permission
   and scope, and one explicitly designated Security Officer. Provision the
   normal two-approval Initialize rule. This tool does not create accounts,
   assign roles or change rules.
2. Prepare an empty, private (0700) operator directory outside the runtime and
   any browser service. Use the utility with `--state /absolute/private/ca
   --socket /absolute/postgres/socket --run RUN --tenant TENANT --event EVENT
   --backend softhsm-test`. If it is an authorized isolated YubiHSM fixture,
   provision that device separately and pass `--backend yubihsm` instead; the
   tool performs no device operations.
3. With these flags, run `create-ca` then
   `preauthorize OFFICER OFFICIAL1 OFFICIAL2`. The CA and numbered CRL are
   short-lived and scoped to this disposable event. The CA private key stays
   in the operator directory; never copy it into the signer or a release
   artifact. Do not run multiple instances with the same CA state directory.
4. In Admin Portal, a listed official opens a live Initialize request for a
   Post and selects **Set up signing** after a fresh Gold login. Setup only
   makes the worker claim and generate the official's signing key, and the
   request still has **zero extra approvals**. Certificate issuance is a
   separate operator action.
5. Inspect the authorized fixture roster. Run `issue-and-attach OFFICER
   OFFICIAL1` for the official, repeating for `OFFICIAL2` once they have
   set up. The utility only issues after stored device CSR proof, exact
   expected key, authorized account, live request and Post match. A different
   CA, attachment replay, or wrong officer fails closed. It rechecks the
   pinned enrollment at the existing officer attachment call. Then return to
   the approval dialog to review and explicitly approve. Refreshing the
   numbered CRL alone uses `publish-crl`.
6. Confirm distinct approvals, one resulting action, and that the audit and
   approval records contain no P12 fallback. A setup stuck pending after
   dispatch is not permission to generate an alternate key or re-run a
   consumed attempt. Inspect the disposable database and device audit.
   Dispose of the whole fixture and CA state after the test, never reuse it
   on another installation.

The fixture authority does not prove that the browser integration works or
that an untested YubiHSM is suitable. The same issuance logic verifies the
CSR independently of the provider, but it relies on the worker's independent
exact-provider and non-exportability admission.

For a two-officer **SoftHSM-only rehearsal**, the operator can start
`serve-preauthorized-pair OFFICER OFFICIAL1 OFFICIAL2 --timeout 900`
after `preauthorize` and before either person opens the dialog. It serves
only those two designated officials using a previously provisioned CA and an
existing unexpired roster. It issues only after each approved first-use action
produces a verified device CSR, then stops. The approving users never need to
run a command, exchange files, or upload a certificate. The process does not
grant eligibility, initiate device operations or approve the voting request.
It refuses hardware, mismatched subjects and ambiguous failures. Renew a
stale CRL explicitly **before** starting any signing ceremony.
