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

```bash
(
set -euo pipefail
cd /root/deploy
D=$(mktemp -d /root/deploy/image-check.XXXXXX)
trap 'rm -rf "$D"' EXIT
blob() {
  local d=$1 f
  [[ $d =~ ^sha256:[0-9a-f]{64}$ ]] || return 1
  f=$D/${d#sha256:}
  [ -s "$f" ] || k3s ctr -n k8s.io content get "$d" > "$f" || return
  [ "$(sha256sum "$f" | cut -d' ' -f1)" = "${d#sha256:}" ] || return 1
  printf '%s\n' "$f"
}
config() {
  local d=$1 f c depth
  for depth in 1 2 3 4; do
    f=$(blob "$d") || return
    c=$(jq -r '.config.digest // empty' "$f") || return
    if [ -n "$c" ]; then
      blob "$c" > /dev/null || return
      printf '%s\n' "$c"
      return
    fi
    d=$(jq -er 'select((.manifests | length) == 1) | .manifests[0].digest' "$f") || return
  done
  return 1
}
grep -E '^(b4|ballot-verifier|braid|harvest|keycloak|windmill) sha256:' image-digests.txt > "$D/expected.txt"
[ "$(wc -l < "$D/expected.txt")" -eq 6 ]
[ "$(cut -d' ' -f1 "$D/expected.txt" | sort -u | wc -l)" -eq 6 ]
k3s kubectl get pods -A -o json > "$D/pods.json"
awk '{print "sequentech.local/" $1 "@" $2}' "$D/expected.txt" > "$D/refs.txt"
jq -e --rawfile refs "$D/refs.txt" '
  ($refs | split("\n") | map(select(length > 0))) as $expected
  | all(.items[]; . as $pod | all(.spec.containers[];
      if (.image | test("^sequentech\\.local/(b4|ballot-verifier|braid|harvest|keycloak|windmill)(@|:)"))
      then . as $container
        | (($expected | index($container.image)) != null)
        and ($pod.status.phase == "Succeeded" or any($pod.status.containerStatuses[]?;
          .name == $container.name and .ready == true and .state.running != null))
      else true end))' "$D/pods.json" > /dev/null
while read -r p d; do
  expected=$(config "$d")
  mapfile -t actual < <(jq -r --arg ref "sequentech.local/$p@$d" '
    .items[] as $pod | $pod.spec.containers[] | select(.image == $ref)
    | .name as $name | $pod.status.containerStatuses[]?
    | select(.name == $name and .ready == true and .state.running != null)
    | .imageID' "$D/pods.json")
  minimum=1; case "$p" in braid|windmill) minimum=2;; esac
  [ "${#actual[@]}" -ge "$minimum" ]
  for id in "${actual[@]}"; do
    observed=$(config "${id##*@}")
    [ "$observed" = "$expected" ]
  done
  printf '%s %s verified\n' "$p" "${#actual[@]}"
done < "$D/expected.txt"
if jq -r '.items[].spec.containers[].image' "$D/pods.json" \
  | grep -E 'sequentech\.local/(b4|ballot|braid|harvest|keycloak|windmill)' \
  | grep -v '@sha256:'; then
  echo 'FAIL: a TOE container uses an unpinned image' >&2
  exit 1
fi
)
```

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

```bash
. /root/deploy/hosts.env
for h in $PORTAL_HOST $ADMIN_HOST; do
  curl -s "https://$h/realms/$R/.well-known/openid-configuration" | jq -r .issuer
done
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  curl -s -o /dev/null -w "$h %{http_code}\n" \
    "https://$h/realms/master/.well-known/openid-configuration"
done
```

Both issuers must be `https://PORTAL_HOST/realms/R` with your values, and the
master realm must answer `403` on all three hosts.

:::warning
The next checks show that the administration portal's client accepts only
the authorization code flow with PKCE, only for its own host, and that sign-in
goes to the external authentication server.
:::

```bash
A="https://$PORTAL_HOST/realms/$R/protocol/openid-connect"
q="client_id=admin-portal&response_type=code&scope=openid"
ru="redirect_uri=https%3A%2F%2F$ADMIN_HOST%2F"
pk="code_challenge=$(printf 'x%.0s' {1..43})&code_challenge_method=S256"
curl -s -o /dev/null -w '%{redirect_url}\n' "$A/auth?$q&$ru&$pk"
curl -s -o /dev/null -w '%{redirect_url}\n' "$A/auth?$q&$ru"
curl -s -o /dev/null -w '%{http_code}\n' \
  "$A/auth?$q&redirect_uri=https%3A%2F%2Fother.invalid%2F&$pk"
curl -s -d grant_type=password -d client_id=admin-portal -d username=x \
  -d password=x "$A/token" | jq -r .error
curl -s -d grant_type=password -d client_id=admin-cli -d username=x \
  -d password=x "$A/token" | jq -r .error
curl -s -d grant_type=password -d client_id=cli-account-admin -d username=x \
  -d password=x "$A/token" | jq -r .error
```

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

```bash
k3s kubectl -n step-infra exec deploy/keycloak -- ls /opt/keycloak/providers
k3s kubectl -n step-infra exec deploy/keycloak -- ls /opt/keycloak/providers \
  | grep -c smart-link-hmac
```

The first command lists the identity service's extension files; none of them
may have `smart-link-hmac` in its name, so the second command must print `0`.
Then check that no realm, client or identity provider uses a flow with the
smart-link authenticator. The identity service gives every realm a built-in
flow `smart-link`, which cannot be deleted and which nothing uses, so the
command counts only the flows that a realm, a client or an identity provider
uses, with the flows they call:

```bash
k3s kubectl -n step-infra exec -i deploy/postgres -- psql -U postgres -tA -d keycloak <<'EOF'
with recursive used(id) as (
  select unnest(array[browser_flow, registration_flow, direct_grant_flow,
    reset_credentials_flow, client_auth_flow, docker_auth_flow]) from realm
  union select value from realm_attribute where name = 'firstBrokerLoginFlowId'
  union select flow_id from client_auth_flow_bindings
  union select first_broker_login_flow_id from identity_provider
  union select post_broker_login_flow_id from identity_provider
  union select e.auth_flow_id from authentication_execution e
    join used u on u.id = e.flow_id)
select count(*) from authentication_execution e join used u on u.id = e.flow_id
  where e.authenticator = 'smart-link'
EOF
```

The command must print `0`.
Then, from the administration workstation:

```bash
. /root/deploy/hosts.env
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  curl -s -o /dev/null -w "$h %{http_code}\n" \
    "https://$h/realms/$R/election/x/public/login"
done
```

All three lines must end in `404`: the identity service has no smart-link
sign-in address.

## GraphQL endpoint and cross-origin requests

On C1:

```bash
k3s kubectl get ingress -A -o json | jq -r '.items[].spec.rules[].http.paths[]
  | select(.backend.service.name == "hasura") | "\(.path) \(.pathType)"' | sort -u
k3s kubectl -n step-apps get deployment hasura -o json \
  | jq -r '.spec.template.spec.containers[].env[]
    | select(.name | test("CONSOLE|DEV_MODE")) | "\(.name) \(.value)"'
```

The first command must print `/hasura/v1/graphql Exact` alone, and the second
`HASURA_GRAPHQL_ENABLE_CONSOLE false` and `HASURA_GRAPHQL_DEV_MODE false`.
From the administration workstation:

```bash
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"type":"export_metadata","args":{}}' "https://$h/hasura/v1/metadata" \
    | grep -c '"code"'
  curl -s -o /dev/null -D - -H 'Origin: https://other.invalid' \
    "https://$h/hasura/v1/graphql" | grep -ci '^access-control-allow-origin'
done
```

Every result must be `0`: the metadata API does not answer, and a foreign
origin gets no cross-origin permission.

## Encrypted secrets and the internal bulletin board

On C1:

```bash
k3s secrets-encrypt status | grep 'Encryption Status'
k3s kubectl -n step-apps get service b4 -o jsonpath='{.spec.type}{"\n"}'
k3s kubectl get ingress -A -o json \
  | jq -r '.items[].spec.rules[].http.paths[].backend.service.name' | grep -cx b4
k3s kubectl get service -A -o json | jq -r '.items[]
  | select(.spec.type != "ClusterIP") | "\(.metadata.namespace)/\(.metadata.name)"'
```

The results must be `Encryption Status: Enabled`, `ClusterIP`, `0` and
`kube-system/traefik` alone.

Check that the application uses the kit's environment-backed secret store and
that its master key is present with the required length. Run this on C1; the
command checks the key inside each container and prints only its result:

```bash
for app in harvest windmill; do
  k3s kubectl -n step-apps exec "deploy/$app" -c "$app" -- sh -eu -c '
    test "${SECRETS_BACKEND:-}" = EnvVarMasterSecret
    test "${#MASTER_SECRET}" -eq 64
    printf "Secret-store configuration: PASS\n"
  ' || exit 1
done
```

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

```bash
k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -d keycloak \
  -tAc "select name from realm where name like '%-event-%'"
```

Then show the sign-in settings of each of these realms, on C1:

```bash
k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -d keycloak -tA \
  -c "select r.name, r.ssl_required, r.sso_idle_timeout, r.events_enabled,
        (select a.value from realm_attribute a where a.realm_id = r.id
          and a.name = 'bruteForceProtected'),
        (select a.value from realm_attribute a where a.realm_id = r.id
          and a.name = 'failureFactor'),
        (select l.value from realm_events_listeners l where l.realm_id = r.id
          and l.value = 'custom-event-listener')
      from realm r where r.name like '%-event-%'"
```

Each line must show the realm name, `EXTERNAL`, `900`, `t`, `true`, the
authentication failure threshold that the administrator set (5 unless the
administrator set another value) and `custom-event-listener`: TLS
required, idle sessions end after 15 minutes, brute-force protection on with
that threshold, and authentication events recorded and passed to the TOE.
Then check each realm listed from the administration workstation:

```bash
E=<name-of-the-election-event-realm>
curl -s "https://$PORTAL_HOST/realms/$E/messaging-channels" | jq -c .code_channels
curl -s "https://$PORTAL_HOST/realms/$E/.well-known/openid-configuration" \
  | jq -r .issuer
curl -s -d grant_type=password -d client_id=admin-cli -d username=x \
  -d password=x "https://$PORTAL_HOST/realms/$E/protocol/openid-connect/token" \
  | jq -r .error
```

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

```bash
. /root/deploy/hosts.env && cd /root/deploy
q() { k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -tA -d "$@"; }
q keycloak -c "select count(*) from keycloak_role r
  where r.name in ('datafix-account', 'voter-manually-verify')
  and (exists (select 1 from user_role_mapping m where m.role_id = r.id)
    or exists (select 1 from group_role_mapping m where m.role_id = r.id)
    or exists (select 1 from composite_role c where c.child_role = r.id))"
q keycloak -c "select count(*) from client
  where client_id in ('ivr-service', 'ivr-voting') and enabled"
grep -rl 'KEYCLOAK_IVR_' kubernetes | wc -l
q hasura -c "select id from sequent_backend.tenant order by id"
q hasura -c "select count(*) from sequent_backend.election_event
  where tenant_id = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5'"
q keycloak -c "select count(*) from user_entity u join realm r on r.id = u.realm_id
  where r.name = '$R' and u.enabled and u.service_account_client_link is null"
q keycloak -c "select r.name, r.admin_events_enabled, r.admin_events_details_enabled
  from realm r where r.name in ('$R', '$CR') order by r.name"
```

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

```bash
cat > /root/deploy/storage-check.yaml <<'EOF'
apiVersion: v1
kind: Pod
metadata:
  name: storage-check
  namespace: step-infra
spec:
  restartPolicy: Never
  containers:
    - name: check
      image: sequentech.local/trustee:latest
      imagePullPolicy: Never
      command: ["/bin/sh", "-c"]
      args:
        - |
          aws s3api list-objects-v2 --bucket public --prefix plugins/ \
            --query 'length(Contents || `[]`)'
          aws --no-sign-request s3api list-objects-v2 \
            --bucket election-event-documents 2>&1 | grep -o AccessDenied
          aws s3api get-bucket-policy --bucket election-event-documents 2>&1 \
            | grep -o NoSuchBucketPolicy
          p='{"Version":"2012-10-17","Statement":[{"Effect":"Allow",'
          p="$p"'"Principal":{"AWS":["*"]},"Action":["s3:GetObject"],'
          p="$p"'"Resource":["arn:aws:s3:::public/*"]}]}'
          aws s3api get-bucket-policy --bucket public --query Policy \
            --output text | tr -d ' \n' | grep -cxF "$p"
          aws s3api get-object-lock-configuration \
            --bucket electoral-log-checkpoints \
            --query ObjectLockConfiguration.ObjectLockEnabled --output text
      env:
        - name: AWS_ENDPOINT_URL
          value: "http://rustfs.step-infra.svc.cluster.local:9000"
        - name: AWS_REGION
          value: "us-east-1"
        - name: AWS_ACCESS_KEY_ID
          value: "admin"
        - name: AWS_SECRET_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: step-secrets
              key: RUSTFS_ROOT_PASSWORD
EOF
k3s kubectl apply -f /root/deploy/storage-check.yaml
k3s kubectl -n step-infra wait pod/storage-check --timeout=2m \
  --for=jsonpath='{.status.phase}'=Succeeded
k3s kubectl -n step-infra logs storage-check
k3s kubectl -n step-infra delete pod storage-check
```

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

```bash
. /root/deploy/hosts.env
E="<names of the election event realms, separated by spaces>"
for r in $R $CR $E; do
  curl -s "https://$PORTAL_HOST/realms/$r/protocol/openid-connect/certs" \
    | jq -r '.keys[].kid'
done > realm-kids.txt
curl -s "https://$PORTAL_HOST/storage/public/certs.json" | jq -r '.keys[].kid' \
  > certs-kids.txt
wc -l < certs-kids.txt
grep -vxFf realm-kids.txt certs-kids.txt
```

The `wc` command must print a number other than `0`, and the `grep` command
must print nothing: the GraphQL engine accepts only keys of these realms.
