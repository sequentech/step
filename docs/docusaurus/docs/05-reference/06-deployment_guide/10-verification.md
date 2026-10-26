---
id: verification
title: Verifying the Installation
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Make every check of this page after the deployment and before anyone else
signs in, and again after every later deployment. If a check fails, the
installation is not in the evaluated configuration: stop and report it to
[security@sequentech.io](mailto:security@sequentech.io).

:::warning
Do not hand the TOE over (see [Self-Test and Handover](11-selftest_handover.md))
until every check on this page has passed.
:::

## Running images

On C1, after importing all delivered images on both nodes, use the verified
`/root/deploy/image-digests.txt`. A runtime image ID may identify an OCI index
instead of the delivered manifest. The following check hashes each referenced
content blob, resolves an index only when it has exactly one child, and
compares the image configuration digest. That configuration binds the image
settings and the root filesystem layer hashes. Each running, ready container
must also name the delivered manifest digest in its pod specification.

<!-- deployment-step: 10-001 -->

**Session: c1.** Run `step 10-001`. Review its implementation with `show 10-001`.

The block must exit successfully and print six `verified` lines: at least one
running container for each TOE part, at least two for `windmill` (the worker
and beat services), and at least two for `braid`, one for each trustee
instance. Every active TOE container must be ready; completed setup jobs may
remain. It also rejects unexpected or tag-only TOE image references. Stop
if a blob is unavailable, an index has multiple children, a digest differs,
or a required container is missing; do not substitute a pod-specification-only
check. The check reads the imported content cache on C1, so keep that cache
until verification completes.

## Version

The first personal administrator signs in to the administration portal as
[The first sign-in](09-accounts.md#the-first-sign-in) describes. The "Version"
in the header must show
`Sequent Voting Platform - uniWAHL Version, v10.0.0`.

![Administration portal with version and source hash in the header](../../02-election_managers/04-administrator_handbook/assets/roles-navigation.png)

Illustrative screen from a synthetic deployment: the header shows version 10.0.0
and its source hash. Compare your actual display with the verified delivery;
this image alone does not establish the installed identity or required role settings.

## TLS endpoint and ingress ports

On C1, `systemctl is-active nginx` must print `active`. Then run the checks of
[Check the TLS profile](05-certificates_tls.md#check-the-tls-profile) from the
administration workstation: for each host the four expected lines of the
first command, every other line `refused`, and port 80 `closed`.

Then repeat the checks of
[Network filtering](04-hosts_network_time.md#network-filtering) from a
computer on a client network, with the ports that C1 prints now. Every line
must say `closed`: the cluster's ingress is not reachable from the network.

## Identity service

From the administration workstation:

<!-- deployment-step: 10-002 -->

**Session: admin.** Run `step 10-002`. Review its implementation with `show 10-002`.

Both issuers must be `https://PORTAL_HOST/realms/R` with your values, and the
master realm must answer `403` on all three hosts.

:::warning
The next checks show that the administration portal's client accepts only
the authorization code flow with PKCE, only for its own host, and that sign-in
goes to the external authentication server.
:::

<!-- deployment-step: 10-003 -->

**Session: admin.** Run `step 10-003`. Review its implementation with `show 10-003`.

The six results must be:

1. an address on PORTAL_HOST that contains `/broker/IDP_ALIAS/login`;
2. an address on ADMIN_HOST that contains `error=invalid_request`, because
   the request has no PKCE challenge;
3. `400`, because the redirect address is not the portal's;
4. `unauthorized_client`, because the client has no password grant;
5. `unauthorized_client`, because the command-line client `admin-cli` has no
   password grant either;
6. `invalid_client`, because the client `cli-account-admin` is disabled.

Once the customer's tenant exists
([Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant)),
run both blocks again with the line `R=$CR` added after
`. /root/deploy/hosts.env`: the results must be the same, with your value of
CR in the issuers.

Then open `https://ADMIN_HOST` in a browser: it must show the sign-in page of
the external authentication server, never a sign-in form of the TOE.

## No smart-link sign-in

:::warning
The identity service must offer no sign-in by link (smart link): such a link
would sign a voter in without the external authentication server.
:::

On C1:

<!-- deployment-step: 10-004 -->

**Session: c1.** Run `step 10-004`. Review its implementation with `show 10-004`.

The first command lists the identity service's extension files; none of them
may have `smart-link-hmac` in its name, so the second command must print `0`.
Then check that no realm, client or identity provider uses a flow with the
smart-link authenticator. The identity service gives every realm a built-in
flow `smart-link`, which cannot be deleted and which nothing uses, so the
command counts only the flows that a realm, a client or an identity provider
uses, with the flows they call:

<!-- deployment-step: 10-005 -->

**Session: c1.** Run `step 10-005`. Review its implementation with `show 10-005`.

The command must print `0`.
Then, from the administration workstation:

<!-- deployment-step: 10-006 -->

**Session: admin.** Run `step 10-006`. Review its implementation with `show 10-006`.

All three lines must end in `404`: the identity service has no smart-link
sign-in address.

## GraphQL endpoint and cross-origin requests

On C1:

<!-- deployment-step: 10-007 -->

**Session: c1.** Run `step 10-007`. Review its implementation with `show 10-007`.

The first command must print `/hasura/v1/graphql Exact` alone, and the second
`HASURA_GRAPHQL_ENABLE_CONSOLE false` and `HASURA_GRAPHQL_DEV_MODE false`.
From the administration workstation:

<!-- deployment-step: 10-008 -->

**Session: admin.** Run `step 10-008`. Review its implementation with `show 10-008`.

Every result must be `0`: the metadata API does not answer, and a foreign
origin gets no cross-origin permission.

## Encrypted secrets and the internal bulletin board

On C1:

<!-- deployment-step: 10-009 -->

**Session: c1.** Run `step 10-009`. Review its implementation with `show 10-009`.

The results must be `Encryption Status: Enabled`, `ClusterIP`, `0` and
`kube-system/traefik` alone.

Check that the application uses the kit's environment-backed secret store and
that its master key is present with the required length. Run this on C1; the
command checks the key inside each container and prints only its result:

<!-- deployment-step: 10-010 -->

**Session: c1.** Run `step 10-010`. Review its implementation with `show 10-010`.

There must be two `Secret-store configuration: PASS` lines and no error.
`SECRETS_BACKEND` is case-sensitive: `env` is not a supported value. If this
check fails, correct the configuration from the installation kit, restart the
affected deployments and repeat it before importing an election event. Keep
the existing master key unchanged: replacing it makes previously encrypted
application data unreadable. Never print the key or enable shell tracing.

## Election event realms

Each election event has its own realm, named `tenant-`, the tenant ID,
`-event-` and the election event's ID. When the election board has imported
the first election event, list the realms of the election events on C1:

<!-- deployment-step: 10-011 -->

**Session: c1.** Run `step 10-011`. Review its implementation with `show 10-011`.

Then show the sign-in settings of each of these realms, on C1:

<!-- deployment-step: 10-012 -->

**Session: c1.** Run `step 10-012`. Review its implementation with `show 10-012`.

Each line must show the realm name, `EXTERNAL`, `900`, `t`, `true`, the
authentication failure threshold that the administrator set (5 unless the
administrator set another value) and `custom-event-listener`: TLS
required, idle sessions end after 15 minutes, brute-force protection on with
that threshold, and authentication events recorded and passed to the TOE.
Then check each realm listed from the administration workstation:

<!-- deployment-step: 10-013 -->

**Session: c1.** Run `step 10-013`. Review its implementation with `show 10-013`.

The first command must print `[]`: voters get no sign-in code and no message
channel, and the voting portal offers no "Contact method". The second must
print `https://PORTAL_HOST/realms/` followed by the realm name. The third must
print `unauthorized_client`: the realm's command-line client `admin-cli` has
no password grant. Repeat these
checks for every election event the election board imports, and tell the
administrator the result. If a check fails, the election event is not in the
evaluated configuration: the election board must not approve its
configuration version, and you report it to
[security@sequentech.io](mailto:security@sequentech.io).

## Data correction, telephone channel, plugins and tenants

:::warning
The admin portal cannot show these settings. The administrator relies on
your written confirmation of them (see
[Self-Test and Handover](11-selftest_handover.md#confirm-the-server-settings-to-the-administrator)).
:::

Make these checks on C1, as root, after the installation and again for each
new election event, before the election board approves its configuration
version:

<!-- deployment-step: 10-014 -->

**Session: c1.** Run `step 10-014`. Review its implementation with `show 10-014`.

The results must be `0` (no account, group or role holds "Datafix account"
or "Manually Verify Voter"),
`0` (the clients of the telephone channel are disabled in every realm), `0`
(no telephone provider credentials are configured), the list of tenants
with exactly two IDs, `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` (the super-admin
tenant of the realm R, which serves only the steps of this guide) and the ID
in your value of CR (the customer's tenant), then `0` (the super-admin tenant
holds no election event), `0` (no personal account of the realm R is
enabled), and the two realms R and CR, each followed by `t|t` (changes made
through the administration interface are recorded with their details).

Then check the buckets of the object storage: no plugin is installed, the
bucket `election-event-documents`, which holds the realm templates, refuses
access without credentials, the bucket `public` lets anyone read its files and
allows nothing else without credentials, and the bucket
`electoral-log-checkpoints`, which holds the checkpoints of the electoral log,
has Object Lock. Plugins would be files under `plugins/` in the bucket
`public`, the only bucket that browsers may read. This one-off pod makes the
checks with the object storage's command-line tool:

<!-- deployment-step: 10-015 -->

**Session: c1.** Run `step 10-015`. Review its implementation with `show 10-015`.

The `logs` command must print five lines: `0` (no plugin), `AccessDenied`
(the bucket `election-event-documents` refuses a request without
credentials), `NoSuchBucketPolicy` (that bucket has no access policy), `1`
(the access policy of `public` is exactly the one in the pod, which lets
anyone read its files and allows nothing else) and `Enabled` (the bucket
`electoral-log-checkpoints` has Object Lock). Never upload a file to the
object storage by hand.

The GraphQL engine accepts every token signed with a key in the file
`certs.json` of the bucket `public`. Check from the administration
workstation that each of these keys is a key of a realm of this installation:
the realm R, the realm CR and the election event realms that you listed in
[Election event realms](#election-event-realms) (none before the first
import).

<!-- deployment-step: 10-016 -->

**Session: c1.** Run `step 10-016`. Review its implementation with `show 10-016`.

The `wc` command must print a number other than `0`, and the `grep` command
must print nothing: the GraphQL engine accepts only keys of these realms.
