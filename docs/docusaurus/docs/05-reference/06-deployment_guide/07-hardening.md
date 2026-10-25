---
id: hardening
title: Configuring the Deployment
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page turns the working copy of the cluster manifests in
`/root/deploy/kubernetes` into the evaluated configuration. Every command runs
on C1, as root, after `. /root/deploy/hosts.env`. Make the sections in order
and each only once; each ends with a check. Several commands change their
files in place or remove their input, so a second run gives a wrong result.
To repeat a section, remove `/root/deploy/kubernetes`, make the working copy
again as [Prepare the deployment directory](03-installation_kit.md#prepare-the-deployment-directory)
describes, and make every section of this page again.

:::warning
Every section of this page is security relevant. Deploy only after all of
them: together they replace the kit's demonstration passwords, run the
identity service in production mode, close the GraphQL console and make the
external authentication server the only way to sign in, as the evaluated
configuration requires.
:::

## Set the host names

:::warning
The host names decide where the identity service sends a browser after
sign-in and which origins may call the TOE. Use exactly the three names of
the certificate.
:::

```bash
cd /root/deploy/kubernetes
grep -rl -e portal.local -e admin.local -e verifier.local . | xargs sed -i \
  -e "s/portal\.local/$PORTAL_HOST/g" -e "s/admin\.local/$ADMIN_HOST/g" \
  -e "s/verifier\.local/$VERIFIER_HOST/g"
grep -rn '\.local\b' . | grep -v -e sequentech.local -e svc.cluster.local
```

The last command must print nothing.

All three portals must reach the identity service under one host name, the
voting portal's:

```bash
sed -i '/name: KEYCLOAK_URL$/{n;s|value: .*|value: "https://'"$PORTAL_HOST"'"|}' \
  admin-portal/deployment.yaml ballot-verifier/deployment.yaml
grep -A1 'name: KEYCLOAK_URL$' */deployment.yaml
```

The three values shown must be `"https://PORTAL_HOST"` with your voting portal
host.

## Set the displayed software identity

The portals display the software release and the Step source revision that
produced the images. Set both from the installed provenance of the verified
installation kit, before the first deployment:

```bash
cd /root/deploy/kubernetes
P=/var/lib/sequent-install/provenance.json
APP_VERSION=$(jq -er '.release | select(type == "string")
  | select(test("^[0-9]+[.][0-9]+[.][0-9]+$"))' "$P") &&
APP_HASH=$(jq -er '.images.manifest.stepRevision | select(type == "string")
  | select(test("^[0-9a-f]{40}$"))' "$P") &&
[ -f config/step-env.yaml ] &&
! grep -qE '^  APP_(VERSION|HASH):' config/step-env.yaml &&
printf '  APP_VERSION: "%s"\n  APP_HASH: "%s"\n' "$APP_VERSION" "$APP_HASH" \
  >> config/step-env.yaml &&
grep -E '^  APP_(VERSION|HASH):' config/step-env.yaml
```

The block must print the two public values. For release `v10.0.0`,
`APP_VERSION` must be `10.0.0`; `APP_HASH` must be the 40-character Step
revision in the verified image provenance. Stop if either value is missing,
or if the block prints nothing. The portal may display the hash in upper
case. Keep the file in YAML format: later sections check its keys.

## Generate the deployment secrets

:::warning
Run this once, before the first deployment. The database creates its roles
with these passwords on its first start, so the file must never be generated
again. Keep `/root/deploy/secrets.env` readable by root only, and never print
it.
:::

Replace the placeholder in the last `echo` with a name for the identity
service's administrator account, used only by the TOE's services:

```bash
cd /root/deploy && umask 077 && set -o noclobber
{
  for v in POSTGRES_PASSWORD RUSTFS_PASSWORD ACTIONS_ADMIN_SECRET MASTER_SECRET \
      HASURA_ADMIN_SECRET QUEUE_DB_OWNER_PASSWORD QUEUE_DB_WORKER_PASSWORD \
      QUEUE_DB_PRODUCER_PASSWORD QUEUE_DB_READER_PASSWORD ELECTORAL_LOG_PG_PASSWORD \
      ELECTORAL_LOG_PG_PROVISIONING_PASSWORD ELECTORAL_LOG_PG_READER_PASSWORD \
      KC_ADMIN_PASSWORD KC_SERVICE_ACCOUNT_SECRET KC_API_KEY_CLIENT_SECRET \
      TRUSTEE1_PASSWORD TRUSTEE2_PASSWORD; do
    echo "export $v=$(openssl rand -hex 32)"
  done
  echo 'export S3_SECRET="$RUSTFS_PASSWORD"'
  echo 'export KC_ADMIN_USER=<name-of-the-service-administrator>'
} > secrets.env
set +o noclobber
stat -c '%a %U' secrets.env      # 600 root
grep -c '^export ' secrets.env   # 19
```

`S3_SECRET` must equal `RUSTFS_PASSWORD`: the services use the object
storage's root account. Every later command that needs a secret loads this
file with `. /root/deploy/secrets.env`.

## Create the service credentials

:::warning
The service administrator account can create and change every realm. Its
password exists only in `secrets.env` and in these two Secrets.
:::

The identity service gets an administrator account in its master realm, which
the TOE's services use to create the realms of election events. The services
also get the secret of their own client in the tenant realm.

```bash
. /root/deploy/secrets.env
k3s kubectl -n step-infra create secret generic keycloak-bootstrap-admin \
  --from-env-file=<(printf '%s\n' "KC_BOOTSTRAP_ADMIN_USERNAME=$KC_ADMIN_USER" \
    "KC_BOOTSTRAP_ADMIN_PASSWORD=$KC_ADMIN_PASSWORD")
k3s kubectl -n step-apps create secret generic keycloak-service-credentials \
  --from-env-file=<(printf '%s\n' KEYCLOAK_CLIENT_ID=service-account \
    "KEYCLOAK_CLIENT_SECRET=$KC_SERVICE_ACCOUNT_SECRET" \
    "KEYCLOAK_ADMIN_CLIENT_ID=$KC_ADMIN_USER" \
    "KEYCLOAK_ADMIN_CLIENT_SECRET=$KC_ADMIN_PASSWORD")
for s in step-infra/keycloak-bootstrap-admin step-apps/keycloak-service-credentials; do
  k3s kubectl -n ${s%/*} get secret ${s#*/} -o jsonpath='{.data}' | jq -r 'keys[]'
done
```

The output must list `KC_BOOTSTRAP_ADMIN_PASSWORD` and
`KC_BOOTSTRAP_ADMIN_USERNAME`, then `KEYCLOAK_ADMIN_CLIENT_ID`,
`KEYCLOAK_ADMIN_CLIENT_SECRET`, `KEYCLOAK_CLIENT_ID` and
`KEYCLOAK_CLIENT_SECRET`. Then give harvest and the two windmill services the
second Secret:

```bash
cd /root/deploy/kubernetes
printf '%s\n' '            - secretRef:' \
  '                name: keycloak-service-credentials' > /root/deploy/envfrom.txt
sed -i '/^                name: step-secrets$/r /root/deploy/envfrom.txt' \
  harvest/deployment.yaml windmill/deployment.yaml windmill/beat.yaml
grep -c 'name: keycloak-service-credentials' harvest/deployment.yaml \
  windmill/deployment.yaml windmill/beat.yaml
```

Each of the three files must count 1.

## Run the identity service in production mode

:::warning
The identity service must run in production mode: only then does it use one
fixed host name and trust the authentication server's CA. Do not deploy
without this section.
:::

The identity service must run in production mode, with one fixed host name,
the trust anchor of the authentication server and the TLS profile for its own
connections. Save this filter as `/root/deploy/keycloak.jq`:

```jq
def without(names): map(select(.name as $n | names | index($n) | not));
"-Djdk.tls.client.protocols=TLSv1.3"
  + " -Djdk.tls.client.cipherSuites=TLS_AES_256_GCM_SHA384"
  + " -Djdk.tls.namedGroups=secp384r1" as $tls
| .spec.template.spec.containers |= map(if .name != "keycloak" then . else
    .args = ["start", "--import-realm"]
    | .env = (.env | without(["KC_HOSTNAME_STRICT", "KC_HOSTNAME_STRICT_HTTPS"]))
      + [{name: "KC_HOSTNAME", value: "https://\($ENV.PORTAL_HOST)"},
         {name: "KC_HTTP_ENABLED", value: "true"},
         {name: "KC_TRUSTSTORE_PATHS",
          value: "/opt/keycloak/conf/truststores/ca.pem"},
         {name: "JAVA_OPTS_APPEND", value: $tls}]
    | .envFrom = (.envFrom // []) + [{secretRef: {name: "keycloak-bootstrap-admin"}}]
    | .volumeMounts += [{name: "auth-server-ca", readOnly: true,
        mountPath: "/opt/keycloak/conf/truststores"}]
  end)
| .spec.template.spec.volumes |= map(if .name == "realm-import"
    then {name, secret: {secretName: "keycloak-realm"}} else . end)
  + [{name: "auth-server-ca", configMap: {name: "auth-server-ca"}}]
```

Then apply it to the manifest, which becomes a JSON file:

```bash
cd /root/deploy/kubernetes/keycloak
k3s kubectl create --dry-run=client -o json -f deployment.yaml \
  | jq -f /root/deploy/keycloak.jq > deployment.json && rm deployment.yaml
jq -r '(.spec.template.spec.containers[] | select(.name == "keycloak")
  | (.args | join(" ")),
    (.env[] | select(.name | test("^KC_HOST|^KC_HTTP_EN|^KC_TRUST|^JAVA")) | .name)),
  (.spec.template.spec.volumes[] | select(.name == "realm-import")
    | .secret.secretName)' deployment.json
```

The output must be `start --import-realm` followed by `KC_HOSTNAME`,
`KC_HTTP_ENABLED`, `KC_TRUSTSTORE_PATHS`, `JAVA_OPTS_APPEND` and
`keycloak-realm`, and no `KC_HOSTNAME_STRICT`. The identity service now reads
the tenant realm from a Secret, which
[Write the documents back](#write-the-documents-back) creates.

## Close the GraphQL console

:::warning
The GraphQL console and the metadata API give full access to the database
schema and its permissions. They must not be reachable.
:::

The GraphQL engine must offer only its GraphQL endpoint: no console, no
development mode, no metadata or other API.

```bash
cd /root/deploy/kubernetes
sed -i -e '/name: HASURA_GRAPHQL_ENABLE_CONSOLE$/{n;s/"true"/"false"/}' \
  -e '/name: HASURA_GRAPHQL_DEV_MODE$/{n;s/"true"/"false"/}' hasura/deployment.yaml
sed -i '/- path: \/hasura$/{s|/hasura$|/hasura/v1/graphql|;n;s/Prefix/Exact/}' \
  admin-portal/ingress.yaml voting-portal/ingress.yaml ballot-verifier/ingress.yaml
grep -A1 -e 'HASURA_GRAPHQL_ENABLE_CONSOLE$' -e 'HASURA_GRAPHQL_DEV_MODE$' \
  hasura/deployment.yaml
grep -A1 'path: /hasura' */ingress.yaml
```

Both values must be `"false"`, and each of the three ingress files must show
`path: /hasura/v1/graphql` with `pathType: Exact`.

## Limit cross-origin access

:::warning
Allow cross-origin requests only from the three portal origins. From any
other origin, a web page on another site could call the TOE with a voter's
browser.
:::

Only the three portal origins may make cross-origin requests:

```bash
printf '      - "https://%s"\n' "$PORTAL_HOST" "$VERIFIER_HOST" "$ADMIN_HOST" \
  > /root/deploy/origins.txt
sed -i -e '/accessControlAllowOriginList:/r /root/deploy/origins.txt' \
  -e '/accessControlAllowOriginList:/{n;/- "\*"/d}' voting-portal/middleware.yaml
grep -A3 'accessControlAllowOriginList:' voting-portal/middleware.yaml
```

The list must hold the three `https://` origins and no `"*"`.

## Block the master realm

:::warning
Without this block, anyone who reaches the portal hosts could try to sign
in to the identity service's master realm.
:::

The identity service's master realm holds its service administrator and must
not be reachable from outside the cluster:

```bash
cat > keycloak/master-blocked.yaml <<EOF
apiVersion: traefik.io/v1alpha1
kind: Middleware
metadata:
  name: deny-all
  namespace: step-infra
spec:
  ipAllowList:
    sourceRange:
      - 192.0.2.1/32
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: keycloak-master-blocked
  namespace: step-infra
  annotations:
    traefik.ingress.kubernetes.io/router.entrypoints: websecure
    traefik.ingress.kubernetes.io/router.middlewares: step-infra-deny-all@kubernetescrd
spec:
  ingressClassName: traefik
  tls:
    - hosts:
        - $PORTAL_HOST
        - $ADMIN_HOST
        - $VERIFIER_HOST
      secretName: step-tls-cert
  rules:
    - host: $PORTAL_HOST
      http: &master-realm
        paths:
          - path: /realms/master
            pathType: Prefix
            backend:
              service:
                name: keycloak
                port:
                  number: 8090
    - host: $ADMIN_HOST
      http: *master-realm
    - host: $VERIFIER_HOST
      http: *master-realm
EOF
grep -E '^    - host:' keycloak/master-blocked.yaml
```

The last command must list the three host names. The only address allowed,
192.0.2.1, is reserved for documentation and is never a client's, so the
ingress refuses every request to the master realm, also those that the TLS
endpoint forwards from C1's loopback interface.
[Verifying the Installation](10-verification.md) checks that the master realm
answers with an error.

## Give harvest its service

The other services reach harvest inside the cluster at port 8400. Add its
Service, reachable only inside the cluster:

```bash
cat > harvest/service.yaml <<'EOF'
apiVersion: v1
kind: Service
metadata:
  name: harvest
  namespace: step-apps
spec:
  selector:
    app: harvest
  ports:
    - port: 8400
      targetPort: 8400
EOF
```

## Keep the results and kiosk portals off

:::warning
Do not set either address, and do not deploy either portal.
:::

No results portal and no kiosk voting portal are part of the evaluated
configuration. Their addresses stay unset:

```bash
grep -nE '^ *(RESULTS_PORTAL_URL|KIOSK_VOTING_PORTAL_URL):' config/step-env.yaml
```

The command must print nothing. The realm settings below also disable the
results portal's client.

## Configure the realms

:::warning
These settings decide who can sign in to the TOE and how. Apply all of them
before the first deployment: the identity service imports the tenant realm
only once.
:::

The identity service imports the tenant realm from the manifest
`keycloak/realm.yaml`. New tenants and election events take their realm from
the two templates in `keycloak/realm-templates.yaml`. All three documents get
the same sign-in settings:

- TLS required for external requests; idle sessions end after 15 minutes;
  brute-force protection locks an account after 5 failed attempts; events
  are recorded and passed to the TOE's event listener;
- no self-registration, password reset or "remember me";
- the external authentication server is the only identity provider, every
  sign-in is redirected to it, the local sign-in forms are disabled, and a
  first sign-in only links to an existing account;
- the administration and voting portals use the authorization code flow with
  PKCE (S256), no password grant, and redirect only to their own hosts; the
  command-line client `admin-cli` has no password grant either.

The tenant realm also gets the operator's client secrets and trustee
passwords, and all demonstration passwords are removed. The seeded
administrator groups (`admin`, `admin-light` and `admin-lockdown`) lose every
permission that the administrator role must not hold, as the role tables of
the [Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md)
list them: the election board's permissions, among them "Create Election
Event" (`election-event-create`), the only permission that the import of
election data needs, the "Sign:" permissions, the scheduled events and the
areas, and every permission that reads, changes or exports election data,
election execution data or audit records. No group holds "Edit Results
Publication" (`publish-results-write`), "Datafix account"
(`datafix-account`) or "Manually Verify Voter" (`voter-manually-verify`).
The election event template also
gets: no sign-in code step and no messaging channel for voters, no account console,
no offline sessions, and the results portal's and the other unused channels'
clients disabled.

Save the list of the permissions that the administrator role must not hold
as `/root/deploy/board.jq`:

```jq
def board_only: test("^((sign|voter|contest|candidate|area|application|document"
  + "|report|miru|tally|publish|logs|electoral-log|signing-requests|monitoring"
  + "|scheduled-event)-.+|election-event-(archive|create|delete|keys-tab"
  + "|logs-tab|publish-(preview|tab|view)|reports-tab|scheduled-(tab|event-columns)"
  + "|tally-tab|tasks-tab|voter-list-reconciliation|voters-tab)|election-(create"
  + "|delete|ip-address-view|publish-tab|state-write|voters-tab)"
  + "|(admin|create|export|transmition|trustee)-ceremony|admin-ip-address-view"
  + "|datafix-account|ee-voters-logs|keys-read|task-export|tasks-read"
  + "|transmission-report-generate|user-import)$");
```

Then save this filter as `/root/deploy/realm.jq`:

```jq
include "board";
def client(id; f): .clients |= map(if .clientId == id then f else . end);
def drop(a; v): if a then a -= [v] else . end;
def portal(uris):
  .publicClient = true | .standardFlowEnabled = true
  | .implicitFlowEnabled = false | .directAccessGrantsEnabled = false
  | .redirectUris = uris | .webOrigins = ["+"]
  | .attributes["pkce.code.challenge.method"] = "S256"
  | .attributes["post.logout.redirect.uris"] = "+";
def execs(f): .authenticationFlows |= map(.authenticationExecutions |= map(f));
def password(v): [{type: "password", value: v, temporary: false}];
($ENV.PORTAL_HOST) as $p | ($ENV.ADMIN_HOST) as $a | ($ENV.VERIFIER_HOST) as $v
| .sslRequired = "external" | .ssoSessionIdleTimeout = 900
| .bruteForceProtected = true | .failureFactor = 5
| .eventsEnabled = true
| .eventsListeners = ((.eventsListeners // []) + ["custom-event-listener"] | unique)
| .registrationAllowed = false | .resetPasswordAllowed = false
| .rememberMe = false
| .identityProviders = $idp | .identityProviderMappers = []
| .authenticatorConfig = [(.authenticatorConfig // [])[]
    | select(.alias != "redirect-to-idp")]
    + [{alias: "redirect-to-idp", config: {defaultProvider: $idp[0].alias}}]
| execs(if .authenticator == "identity-provider-redirector"
          then .authenticatorConfig = "redirect-to-idp"
        elif .flowAlias == "sequent browser flow forms"
          or .flowAlias == "message channel OTP"
          then .requirement = "DISABLED"
        else . end)
| if any(.authenticationFlows[]; .alias == "certificate-first-login-flow") then .
  else .authenticationFlows += [$flow[0] | del(.id)] end
| client("admin-cli"; .directAccessGrantsEnabled = false)
| .authenticationFlows |= map(select(.alias != "smart-link" and .alias != "smart-link forms"))
| if $doc == "event-template" then
    client("voting-portal"; portal(["https://\($p)/*", "https://\($v)/*"]))
    | reduce ("account-console", "results-portal", "onsite-voting-portal",
        "vp-sso", "ivr-service", "ivr-voting", "voting-portal-certs") as $c
        (.; client($c; .enabled = false))
    | del(.attributes["sequent.messaging"])
    | .defaultRole.name as $dr
    | .roles.realm |= map(if .name == $dr
        then drop(.composites.realm; "offline_access") else . end)
    | drop(.defaultOptionalClientScopes; "offline_access")
    | .clients |= map(drop(.optionalClientScopes; "offline_access"))
  else
    client("admin-portal"; portal(["https://\($a)/*"]))
    | client("voting-portal"; portal(["https://\($p)/*", "https://\($v)/*"]))
    | client("api-key-client"; del(.authenticationFlowBindingOverrides))
    | client("cli-account-admin"; .enabled = false | .directAccessGrantsEnabled = false)
    | .adminEventsEnabled = true | .adminEventsDetailsEnabled = true
    | .groups |= map(drop(.realmRoles; "publish-results-write")
        | drop(.realmRoles; "datafix-account")
        | drop(.realmRoles; "voter-manually-verify")
        | if (.name | startswith("admin")) and .realmRoles
          then .realmRoles |= map(select(board_only | not)) else . end)
    | if $doc == "tenant" then
        client("service-account"; .secret = $ENV.KC_SERVICE_ACCOUNT_SECRET)
        | client("api-key-client"; .secret = $ENV.KC_API_KEY_CLIENT_SECRET)
        | .users |= map(
            if .username == "trustee1"
              then .credentials = password($ENV.TRUSTEE1_PASSWORD)
            elif .username == "trustee2"
              then .credentials = password($ENV.TRUSTEE2_PASSWORD)
            else del(.credentials) end)
      elif .users then .users |= map(del(.credentials))
      else . end
  end
```

Extract the three documents and apply the filter to each:

```bash
. /root/deploy/hosts.env && . /root/deploy/secrets.env
cd /root/deploy/kubernetes/keycloak
k3s kubectl create --dry-run=client -o json -f realm.yaml > ../../realm.cm.json
k3s kubectl create --dry-run=client -o json -f realm-templates.yaml \
  | jq -s '{apiVersion: "v1", kind: "List", items: .}' > ../../templates.cm.json
cd /root/deploy
T=$R.json
E=$R-event-33f18502-a67c-4853-8333-a58630663559.json
pick() { jq -r --arg n "$1" --arg k "$2" \
  '.items[] | select(.metadata.name == $n) | .data[$k]' templates.cm.json; }
jq -r --arg k "$T" '.data[$k]' realm.cm.json > realm-tenant.json
pick keycloak-tenant-realm-template "$T" > realm-tenant-template.json
pick keycloak-election-event-realm-template "$E" > realm-event-template.json
jq '.authenticationFlows[] | select(.alias == "certificate-first-login-flow")' \
  realm-event-template.json > flow.json
for d in tenant tenant-template event-template; do
  jq -e -L /root/deploy --arg doc $d --slurpfile idp idp.json \
    --slurpfile flow flow.json -f realm.jq realm-$d.json > realm-$d.new.json && echo "$d done"
done
```

The loop must print `done` for the three documents. Then check them:

```bash
for d in tenant tenant-template event-template; do
  jq -c '{sslRequired, ssoSessionIdleTimeout, bruteForceProtected, failureFactor,
    eventsEnabled, registrationAllowed, resetPasswordAllowed, rememberMe,
    idps: [.identityProviders[].alias],
    users: [.users[]? | select(.credentials) | .username],
    disabled: [.clients[] | select(.enabled == false) | .clientId]}' \
    realm-$d.new.json
done
```

Each line must show `"sslRequired":"external"`, `"ssoSessionIdleTimeout":900`,
`"bruteForceProtected":true`, `"failureFactor":5`, `"eventsEnabled":true`,
`false` for the three self-service settings, and IDP_ALIAS as the only
identity provider. Only the tenant document lists users with a credential:
`trustee1` and `trustee2`. The tenant document and the tenant template list
the disabled client `cli-account-admin`; the event template lists
`account-console`, `ivr-service`, `ivr-voting`, `onsite-voting-portal`,
`results-portal`, `vp-sso` and `voting-portal-certs`.

Check the sign-in settings in detail:

```bash
for d in tenant tenant-template event-template; do
  jq -L /root/deploy -c 'include "board";
    {listener: (.eventsListeners | index("custom-event-listener") != null),
    results: [.groups[]?.realmRoles[]?, .users[]?.realmRoles[]?
      | select(. == "publish-results-write" or . == "datafix-account"
        or . == "voter-manually-verify")],
    admin_groups: [.groups[]? | select(.name | startswith("admin")) | .realmRoles[]?
      | select(board_only)],
    forms: [.authenticationFlows[].authenticationExecutions[]
      | select(.flowAlias == "sequent browser flow forms") | .requirement],
    redirect: [.authenticatorConfig[] | select(.alias == "redirect-to-idp")
      | .config.defaultProvider],
    portals: [.clients[] | select(.clientId | test("^(admin|voting)-portal$"))
      | [.clientId, .directAccessGrantsEnabled,
         .attributes["pkce.code.challenge.method"]]],
    admin_cli: [.clients[] | select(.clientId == "admin-cli")
      | .directAccessGrantsEnabled],
    cli_account_admin: [.clients[] | select(.clientId == "cli-account-admin")
      | [.enabled, .directAccessGrantsEnabled]],
    smart_link: [.authenticationFlows[] | select((.alias | startswith("smart-link"))
      or any(.authenticationExecutions[]; .authenticator == "smart-link")) | .alias],
    admin_events: [.adminEventsEnabled, .adminEventsDetailsEnabled]}' realm-$d.new.json
done
jq -c '.defaultRole.name as $dr
  | {messaging: (.attributes["sequent.messaging"] // "absent"),
  code_step: [.authenticationFlows[].authenticationExecutions[]
    | select(.flowAlias == "message channel OTP") | .requirement],
  offline: [.defaultOptionalClientScopes[], .clients[].optionalClientScopes[]?,
    (.roles.realm[] | select(.name == $dr) | .composites.realm[]?)]
    | index("offline_access")}' realm-event-template.new.json
```

For each document, `listener` must be `true`, `results` (no group or account
with "Edit Results Publication", "Datafix account" or "Manually Verify
Voter") and `admin_groups` (no excluded permission left in an administrator
group) empty, every `forms` entry `DISABLED`, `redirect` IDP_ALIAS, each portal
client must show `false` and `"S256"`, `admin_cli` must be `[false]` and
`smart_link` (no sign-in flow with the smart-link authenticator) empty. For the
tenant document and the tenant template, `cli_account_admin` must be
`[[false,false]]` (the command-line account client is disabled and has no
password grant) and `admin_events` `[true,true]` (changes made through the
administration interface are recorded with their details); for the event
template they are `[]` and `[false,false]`. The last command must print
`{"messaging":"absent","code_step":["DISABLED"],"offline":null}`.

### Add the installation account

The tenant of this realm is the super-admin tenant. It serves only the steps
of this guide and holds no election work. The identity service imports its
realm only at its first start, so the installation account is added to the
tenant document now. The operator holds it and uses it only to create the
customer's tenant
([Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant)),
which disables it afterwards. It gets the group and roles of the seeded
administrator account, no password, and the user name and e-mail address that
the external authentication server holds for the operator:

```bash
cd /root/deploy
jq --arg u '<operator-user-name-at-the-authentication-server>' --arg m '<e-mail-address>' \
  '.users += [.users[] | select(.username == "admin")
    | {username: $u, email: $m, emailVerified, enabled: true, realmRoles, groups,
       attributes: {"tenant-id": .attributes["tenant-id"]}}]' \
  realm-tenant.new.json > realm-tenant.tmp && mv realm-tenant.tmp realm-tenant.new.json
jq -c '.users[] | select(.groups == ["/admin"]) | [.username, (.credentials != null)]' \
  realm-tenant.new.json
```

The last command must list `admin`, `api-user` and the new user name, each
with `false`.

### Write the documents back

:::warning
The documents hold client secrets, the trustee passwords and the external
authentication server's client secret. They go into the cluster only as
Secrets, which the cluster stores encrypted, never as ConfigMaps.
:::

Write the documents back into the manifests as Secrets, and let the setup job
of the object storage read the templates from them:

```bash
cd /root/deploy
jq --arg k "$T" --rawfile r realm-tenant.new.json \
  '.kind = "Secret" | .type = "Opaque" | del(.data) | .stringData = {($k): $r}' \
  realm.cm.json > kubernetes/keycloak/realm.json
jq --arg t "$T" --arg e "$E" --rawfile tt realm-tenant-template.new.json \
  --rawfile et realm-event-template.new.json \
  '.items |= map(.kind = "Secret" | .type = "Opaque" | del(.data)
    | .stringData = if .metadata.name == "keycloak-tenant-realm-template"
        then {($t): $tt} else {($e): $et} end)' \
  templates.cm.json > kubernetes/keycloak/realm-templates.json
sed -i 's/^              - configMap:$/              - secret:/' \
  kubernetes/rustfs/buckets.yaml
rm kubernetes/keycloak/realm.yaml kubernetes/keycloak/realm-templates.yaml
chmod -R go-rwx /root/deploy
ls kubernetes/keycloak
jq -r '.kind' kubernetes/keycloak/realm.json
jq -r '.items[] | "\(.kind) \(.metadata.name)"' kubernetes/keycloak/realm-templates.json
grep -A1 -e '- secret:' -e '- configMap:' kubernetes/rustfs/buckets.yaml
grep -rl 'kind: ConfigMap' kubernetes | xargs grep -il -e secret -e password
```

The `ls` command must list `deployment.json`, `ingress.yaml`,
`master-blocked.yaml`, `realm-templates.json`, `realm.json` and
`service.yaml`. The next commands must print `Secret`, then
`Secret keycloak-tenant-realm-template` and
`Secret keycloak-election-event-realm-template`, then two `- secret:` lines,
each followed by the name of one template. The last command lists the
manifests with ConfigMaps that contain the words "secret" or "password": it
may list only `config/step-env.yaml` and `postgres/init-script.yaml`, which
name passwords and Secrets but hold no secret value.

The Secret `keycloak-realm` holds the operator's client secrets and the
trustee passwords only until
[Installing the TOE](08-installing_toe.md#remove-the-secrets-from-the-realm-manifest)
removes them after the first start. Do not copy the deployment directory
anywhere.
