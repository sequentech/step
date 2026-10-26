---
id: verification
title: Verifying the Installation
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Read this chapter for the requirements and expected results behind the
[configuration-driven installation](01-about.md#install-from-one-configuration-file).
Operation references identify implementation details and troubleshooting checks;
they are not additional shell commands to type after a successful installation.

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

**Operation reference:** `10-verify-running-images` on `c1`.

**Expected result.** Six lines ending in `verified`, with the minimum ready-container counts described below; no error.

**If the result differs.** A missing blob, ambiguous index or unequal digest means verification failed. Check the delivered cache and pod readiness, then repeat this read-only check; do not accept only the pod specification as proof.


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

**Operation reference:** `10-check-login-discovery` on `admin`.

**Expected result.** After choosing `installation` at the realm prompt, two matching issuer URLs on the voting portal host, then three master-realm responses ending in `403`. Select `customer` when repeating this check for that tenant.

**If the result differs.** A `000` status, blank issuer or TLS error is a connectivity failure, not a successful denial. Check DNS and certificate trust first. Any reachable master realm requires correcting the ingress before handover.


Both issuers must be `https://PORTAL_HOST/realms/R` with your values, and the
master realm must answer `403` on all three hosts.

:::warning
The next checks show that the administration portal's client accepts only
the authorization code flow with PKCE, only for its own host, and that sign-in
goes to the external authentication server.
:::

<!-- deployment-step: 10-003 -->

**Operation reference:** `10-check-login-redirects` on `admin`.

**Expected result.** After choosing the same realm as the preceding check, six results in the order below: IdP redirect, PKCE error redirect, `400`, two `unauthorized_client` responses and `invalid_client`.

**If the result differs.** Blank results, network errors or successful password-grant responses fail this check. Check the correct realm and its imported client/IdP settings; do not enable a password grant to continue.


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
the same discovery and redirect conditions must also hold for its realm. The
issuer must name the customer realm, and all six redirect/grant results above
must match. Check both realm results during commissioning; an installation-realm
result alone does not establish the customer realm's configuration. Do not edit
the bundled scripts or substitute an unrelated realm to obtain a passing result.

Then open `https://ADMIN_HOST` in a browser: it must show the sign-in page of
the external authentication server, never a sign-in form of the TOE.

## No smart-link sign-in

:::warning
The identity service must offer no sign-in by link (smart link): such a link
would sign a voter in without the external authentication server.
:::

On C1:

<!-- deployment-step: 10-004 -->

**Operation reference:** `10-check-login-providers` on `c1`.

**Expected result.** The provider file listing contains no `smart-link-hmac` file; its final count is `0`.

**If the result differs.** A nonzero count means the wrong identity-service build or extension set is installed. Check the delivered image identity; do not manually delete files inside a running container.


The first command lists the identity service's extension files; none of them
may have `smart-link-hmac` in its name, so the second command must print `0`.
Then check that no realm, client or identity provider uses a flow with the
smart-link authenticator. The identity service gives every realm a built-in
flow `smart-link`, which cannot be deleted and which nothing uses, so the
command counts only the flows that a realm, a client or an identity provider
uses, with the flows they call:

<!-- deployment-step: 10-005 -->

**Operation reference:** `10-check-login-flows` on `c1`.

**Expected result.** Exactly `0`: no used authentication flow invokes smart-link.

**If the result differs.** A nonzero result or SQL error is not acceptable. Check the imported realm templates and actual used flows; do not delete built-in flows or alter the database directly.


The command must print `0`.
Then, from the administration workstation:

<!-- deployment-step: 10-006 -->

**Operation reference:** `10-check-public-login-disabled` on `admin`.

**Expected result.** Three lines, one per portal host, each ending in `404`.

**If the result differs.** A `000` response indicates a network/TLS problem, not a blocked endpoint. Restore connectivity and repeat; any reachable smart-link endpoint blocks handover.


All three lines must end in `404`: the identity service has no smart-link
sign-in address.

## GraphQL endpoint and cross-origin requests

On C1:

<!-- deployment-step: 10-007 -->

**Operation reference:** `10-check-graphql-settings` on `c1`.

**Expected result.** `/hasura/v1/graphql Exact`, followed by `HASURA_GRAPHQL_ENABLE_CONSOLE false` and `HASURA_GRAPHQL_DEV_MODE false`.

**If the result differs.** If a broader path, console or development mode is present, correct the protected manifests and redeploy through the documented procedure. Do not expose the console for diagnosis.


The first command must print `/hasura/v1/graphql Exact` alone, and the second
`HASURA_GRAPHQL_ENABLE_CONSOLE false` and `HASURA_GRAPHQL_DEV_MODE false`.
From the administration workstation:

<!-- deployment-step: 10-008 -->

**Operation reference:** `10-check-metadata-and-cors` on `admin`.

**Expected result.** Six `0` results: two checks for each portal host. There must also be no curl/TLS error; an unreachable server is not proof that access is restricted.

**If the result differs.** A nonzero result means metadata or foreign-origin access is exposed. Check the ingress exact path and both CORS allowlists, redeploy through the guide, and repeat before handover.


Every result must be `0`: the metadata API does not answer, and a foreign
origin gets no cross-origin permission.

## Encrypted secrets and the internal bulletin board

On C1:

<!-- deployment-step: 10-009 -->

**Operation reference:** `10-check-service-isolation` on `c1`.

**Expected result.** `Encryption Status: Enabled`, `ClusterIP`, `0`, and only `kube-system/traefik` in the external-service list.

**If the result differs.** If encryption is disabled, the bulletin board has ingress, or another service is externally exposed, stop. Check the verified cluster and service configuration; do not create more Secrets on an unencrypted cluster.


The results must be `Encryption Status: Enabled`, `ClusterIP`, `0` and
`kube-system/traefik` alone.

Check that the application uses the kit's environment-backed secret store and
that its master key is present with the required length. Run this on C1; the
command checks the key inside each container and prints only its result:

<!-- deployment-step: 10-010 -->

**Operation reference:** `10-check-secret-store` on `c1`.

**Expected result.** Two identical `Secret-store configuration: PASS` lines. No key value is shown.

**If the result differs.** If either is missing, check `SECRETS_BACKEND` and the existing master key configuration as described below. Never generate a replacement master key or print it to diagnose this failure.


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

**Operation reference:** `10-list-event-realms` on `c1`.

**Expected result.** One realm name per imported election event. Before the first import, an empty result is expected, but it does not complete the required post-import checks.

**If the result differs.** If an imported event's realm is absent or an unfamiliar realm appears, compare with the board's import record. Do not approve the event configuration until the mismatch is resolved.


Then show the sign-in settings of each of these realms, on C1:

<!-- deployment-step: 10-012 -->

**Operation reference:** `10-check-live-realm-policy` on `c1`.

**Expected result.** One row per event realm with the TLS, timeout, brute-force and event-listener values described below.

**If the result differs.** If a row is absent or a security setting differs, hold the board's configuration approval. Check the imported event realm against its hardened template; do not dismiss an empty result as a pass.


Each line must show the realm name, `EXTERNAL`, `900`, `t`, `true`, the
authentication failure threshold that the administrator set (5 unless the
administrator set another value) and `custom-event-listener`: TLS
required, idle sessions end after 15 minutes, brute-force protection on with
that threshold, and authentication events recorded and passed to the TOE.
The check must cover each listed realm on C1:

<!-- deployment-step: 10-013 -->

**Operation reference:** `10-check-event-login-policy` on `c1`.

**Expected result.** Three results for the selected realm: `[]`, its HTTPS issuer URL and `unauthorized_client`.

**If the result differs.** If an issuer is missing, a message channel appears or password login succeeds, do not let voters use that event. Check the selected realm name and its import configuration and tell the administrator.


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

**Operation reference:** `10-check-prohibited-roles` on `c1`.

**Expected result.** The zero counts, exactly two tenant IDs, and `t|t` administration-audit flags described below.

**If the result differs.** Any extra tenant, enabled installation account or forbidden permission blocks handover. Compare with the customer setup and role assignments; do not delete database rows or unrelated accounts to force the counts.


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

**Operation reference:** `10-check-storage-policy` on `c1`.

**Expected result.** The temporary check pod completes; its five log values are `0`, `AccessDenied`, `NoSuchBucketPolicy`, `1`, and `Enabled`, then the pod is deleted.

**If the result differs.** A timeout, missing line or other policy result is a failure. Inspect the check pod's operating log and kit storage setup; do not upload or remove objects by hand or accept public access to private buckets.


The `logs` command must print five lines: `0` (no plugin), `AccessDenied`
(the bucket `election-event-documents` refuses a request without
credentials), `NoSuchBucketPolicy` (that bucket has no access policy), `1`
(the access policy of `public` is exactly the one in the pod, which lets
anyone read its files and allows nothing else) and `Enabled` (the bucket
`electoral-log-checkpoints` has Object Lock). Never upload a file to the
object storage by hand.

The GraphQL engine accepts every token signed with a key in the file
`certs.json` of the bucket `public`. Check from the administration
workstation or C1 that each of these keys is a key of a realm of this installation.
The bundled step below runs on C1:
the realm R, the realm CR and the election event realms that you listed in
[Election event realms](#election-event-realms) (none before the first
import).

<!-- deployment-step: 10-016 -->

**Operation reference:** `10-check-published-signing-keys` on `c1`.

**Expected result.** A nonzero count of published keys and no subsequent unmatched key ID.

**If the result differs.** If the count is zero, first check retrieval and realm names. If an unmatched key appears, compare all published keys with the installation's known realms; withhold handover and do not edit `certs.json` by hand.


The `wc` command must print a number other than `0`, and the `grep` command
must print nothing: the GraphQL engine accepts only keys of these realms.
