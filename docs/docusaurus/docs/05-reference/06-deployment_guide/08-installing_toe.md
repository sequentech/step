---
id: installing_toe
title: Installing the TOE
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page loads the TOE images from the verified delivery medium onto both
servers, pins every TOE container to its delivered digest, deploys, and
installs the trustees' keys. It also covers the trustee workstations, which
run the trustee tool from the same delivery.

:::warning
Use only the delivery medium that passed every step of
[Verifying the Delivery](../08-delivery_verification.md). The installation kit
also carries images under the same names; the TOE parts must never run from
those.
:::

## Load the TOE images

On the administration workstation, mount the delivery medium read-only with
the commands of step 3 of [Verifying the Delivery](../08-delivery_verification.md),
then copy the six images that run on the servers to C1 and C2:

```bash
. /root/deploy/hosts.env
M=/mnt/temenos-delivery
for n in "$C1" "$C2"; do
  ssh root@$n 'install -d -m 700 /root/toe-images'
  scp "$M/image-digests.txt" root@$n:/root/toe-images/
  for p in b4 ballot-verifier braid harvest keycloak windmill; do
    tar -C "$M/images/$p" -cf - . | ssh root@$n "cat > /root/toe-images/$p.tar"
  done
done
umount "$M" && cryptsetup close temenos-delivery
```

On C1, and then on C2, import them and check that each one is present under
its delivered digest:

```bash
cd /root/toe-images
for p in b4 ballot-verifier braid harvest keycloak windmill; do
  k3s ctr -n k8s.io images import --base-name sequentech.local/$p --digests $p.tar
done
grep -E '^(b4|ballot-verifier|braid|harvest|keycloak|windmill) sha256:' \
  image-digests.txt | while read -r p d; do
  k3s ctr -n k8s.io images ls -q | grep -qx "sequentech.local/$p@$d" \
    && echo "$p loaded" || echo "$p MISSING"
done
```

The last command must print `loaded` for all six images on both servers. If
one is `MISSING`, do not continue.

## Pin the TOE images

:::warning
Every TOE container must name its image by the digest of the delivery. A
container that names an image by a tag can run the kit's image of the same
name.
:::

On C1:

```bash
cd /root/deploy/kubernetes
cp /root/toe-images/image-digests.txt /root/deploy/
d() { sed -n "s/^$1 \(sha256:[0-9a-f]*\)\$/\1/p" /root/deploy/image-digests.txt; }
for p in harvest windmill ballot-verifier; do
  grep -rl --include=*.yaml "sequentech.local/$p:latest" . | xargs sed -i \
    "s|sequentech.local/$p:latest|sequentech.local/$p@$(d $p)|"
done
sed -i "s|sequentech.local/b4v6:latest|sequentech.local/b4@$(d b4)|" b4/deployment.yaml
sed -i "s|sequentech.local/trustee:latest|sequentech.local/braid@$(d braid)|" \
  trustees/deployment.yaml
sed -i "s|\"sequentech.local/keycloak\"|\"sequentech.local/keycloak@$(d keycloak)\"|" \
  keycloak/deployment.json
grep -rhoE 'sequentech\.local/(b4|ballot|braid|harvest|keycloak|trustee|windmill)[^" ]*' \
  . | sed 's/@sha256:.*/@sha256:.../' | sort | uniq -c
```

The output must be exactly:

```text
      1 sequentech.local/b4@sha256:...
      1 sequentech.local/ballot-verifier@sha256:...
      2 sequentech.local/braid@sha256:...
      1 sequentech.local/harvest@sha256:...
      1 sequentech.local/keycloak@sha256:...
      2 sequentech.local/trustee:latest
      3 sequentech.local/windmill@sha256:...
```

The two `trustee:latest` references are the setup steps of the bulletin board
and of the object storage, which use the tools of the kit's image and run no
TOE function.

## Deploy

:::warning
Load `secrets.env` in the same shell before every deployment. A deployment
without it sets demonstration passwords on the databases and services.
:::

On C1:

```bash
. /root/deploy/hosts.env && . /root/deploy/secrets.env
[ -n "$POSTGRES_PASSWORD" ] && sequent deploy --manifests /root/deploy/kubernetes
k3s kubectl -n step-infra rollout status deployment/keycloak --timeout=15m
k3s kubectl get pods -A
```

The deployment must print `step-tls-cert already exists; keeping it.` and end
without an error. After a few minutes every pod must be `Running` or
`Completed`, except `trustee1` and `trustee2`: they stay not ready until their
keys are installed below. Run the same commands again after every later change
to `/root/deploy/kubernetes`.

## Remove the secrets from the realm manifest

:::warning
The identity service has now imported the tenant realm. Its manifest and the
Secret `keycloak-realm` still hold the client secrets and the trustee
passwords, and must no longer.
:::

On C1:

```bash
. /root/deploy/hosts.env && cd /root/deploy
T=$R.json
curl -s -o /dev/null -w '%{http_code}\n' "https://$PORTAL_HOST/realms/$R"    # 200
jq '(.clients[] | select(.clientId == "service-account") | .secret)
    = "CHANGE_ME_SERVICE_ACCOUNT_SECRET"
  | (.clients[] | select(.clientId == "api-key-client") | .secret)
    = "CHANGE_ME_API_KEY_CLIENT_SECRET"
  | .users |= map(del(.credentials))
  | .identityProviders |= map(del(.config.clientSecret))' realm-tenant.new.json \
  > realm-tenant.clean.json
jq --arg k "$T" --rawfile r realm-tenant.clean.json '.stringData[$k] = $r' \
  kubernetes/keycloak/realm.json > realm.tmp && mv realm.tmp kubernetes/keycloak/realm.json
k3s kubectl apply -f kubernetes/keycloak/realm.json
shred -u realm-tenant.new.json
jq -r --arg k "$T" '.stringData[$k]' kubernetes/keycloak/realm.json | jq -c '{
  credentials: [.users[] | select(.credentials)] | length,
  idp_secret: [.identityProviders[].config.clientSecret // empty] | length,
  secrets: [.clients[] | select(.clientId == "service-account"
    or .clientId == "api-key-client") | .secret]}'
```

The last command must print
`{"credentials":0,"idp_secret":0,"secrets":["CHANGE_ME_API_KEY_CLIENT_SECRET","CHANGE_ME_SERVICE_ACCOUNT_SECRET"]}`.
The running realm keeps the values set at its import. Check with
`k3s kubectl get pods -A` that the pods are still running.

## Create the customer's tenant

:::warning
All election work happens in the customer's tenant. The tenant of the realm R
is the super-admin tenant: it serves only the steps of this guide, holds no
election event, and none of its personal accounts stays enabled.
:::

The operator creates the customer's tenant once, after [Deploy](#deploy) and
before the trustee keys are installed:

1. Open `https://ADMIN_HOST` on the administration workstation and sign in at
   the external authentication server as the installation account of
   [Add the installation account](07-hardening.md#add-the-installation-account).
2. In the menu, next to the tenant's name, select the plus button "Create new
   tenant". Enter the customer's short name in the field "Slug" and select
   "Save". The TOE creates the tenant and its realm from the tenant template
   that [Configure the realms](07-hardening.md#configure-the-realms) has
   hardened, with the service accounts and a seeded `admin` account without a
   password.
3. On C1, list the tenants and record the realm name of the customer's tenant:

   ```bash
   . /root/deploy/hosts.env && cd /root/deploy
   k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -tA \
     -d hasura -c "select id, slug from sequent_backend.tenant order by slug"
   echo 'export CR=tenant-<id-of-the-customer-tenant>' >> /root/deploy/hosts.env
   ```

   The first command must list two tenants: `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`
   (the super-admin tenant) and the customer's tenant with the short name you
   entered. Put its ID into the second command.
4. Ask the operator of the external authentication server to register the
   redirect URI `https://PORTAL_HOST/realms/CR/broker/IDP_ALIAS/endpoint`,
   with your value of CR (see
   [Connecting the Authentication Server](06-authentication_server.md)).
5. On C1, add the customer's first administrator and the two trustee accounts
   to the realm CR, give its client `api-key-client` the trustees' client
   secret, and disable every personal account of the realm R. The
   identity service's own administration tool does this inside the identity
   service; replace the two placeholders with the user name and e-mail address
   that the external authentication server holds for the customer's first
   administrator:

   ```bash
   . /root/deploy/hosts.env && . /root/deploy/secrets.env
   kc() { k3s kubectl -n step-infra exec -i deploy/keycloak -- \
     /opt/keycloak/bin/kcadm.sh "$@" --config /tmp/kcadm.config; }
   kc config credentials --server http://localhost:8090 --realm master \
     --user "$KC_ADMIN_USER" --password "$KC_ADMIN_PASSWORD"
   U='<user-name-at-the-authentication-server>'
   jq -n --arg u "$U" --arg m '<e-mail-address>' --arg t "${CR#tenant-}" \
     '{username: $u, email: $m, emailVerified: true, enabled: true,
       groups: ["/admin"], attributes: {"tenant-id": [$t]}}' \
     | kc create users -r "$CR" -f -
   kc add-roles -r "$CR" --uusername "$U" --rolename permission-label-write
   for N in 1 2; do
     jq -n --arg n trustee$N --arg v TRUSTEE${N}_PASSWORD --arg t "${CR#tenant-}" \
       '{username: $n, enabled: true, groups: ["/trustee"],
         attributes: {"tenant-id": [$t], trustee: [$n]},
         credentials: [{type: "password", value: $ENV[$v], temporary: false}]}' \
       | kc create users -r "$CR" -f -
     kc add-roles -r "$CR" --uusername trustee$N \
       --rolename trustee-read --rolename trustee-write
   done
   K=$(kc get clients -r "$CR" -q clientId=api-key-client --fields id \
     --format csv --noquotes)
   jq -n '{secret: $ENV.KC_API_KEY_CLIENT_SECRET}' \
     | kc update clients/$K -r "$CR" --merge -f -
   kc get users -r "$R" --fields id,username --format csv --noquotes \
     | grep -v ',service-account-' | cut -d, -f1 | while read -r I; do
       kc update users/$I -r "$R" -s enabled=false < /dev/null; done
   kc get users -r "$CR" --fields username,enabled --format csv --noquotes
   kc get users -r "$R" --fields username,enabled --format csv --noquotes
   k3s kubectl -n step-infra exec deploy/keycloak -- rm /tmp/kcadm.config
   ```

   For the realm CR the list must show `admin`, the first administrator's
   user name, `trustee1` and `trustee2`, each with `true`. For the realm R
   every line must end in `false`. Neither list shows the accounts whose names
   start with `service-account-`: the identity service does not list them, and
   they stay enabled.
   The installation account is now disabled: no later step of this guide
   needs an account of the super-admin tenant.
6. Make the checks of
   [Identity service](10-verification.md#identity-service) with
   the line `R=$CR` added after `. /root/deploy/hosts.env`, and of
   [Data correction, telephone channel, plugins and tenants](10-verification.md#data-correction-telephone-channel-plugins-and-tenants).

## Install the trustee keys

Each trustee generates their keys on a trustee workstation (see
[Preparing a trustee workstation](#preparing-a-trustee-workstation) and the
trustee procedures of the
[Handbook for Election Organizers (AGD-03)](../../02-election_managers/05-election_organizer_handbook/01-about.md)),
and the administrators register the public keys as the section "Registering
the trustees' public keys" of the
[Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md)
describes. The trustee then hands over the keys file for their instance of
the trustee server: `trustee1` for the first trustee, `trustee2` for the
second.

:::warning
The keys file holds the trustee's private keys. Take it over only in person,
with the trustee watching every step, directly from the trustee's own medium.
No copy may remain on the administration workstation or on C1: it is kept
only in the cluster's encrypted Secret.
:::

On the administration workstation, with the trustee's medium mounted, set N
to the trustee's instance number and the path to their keys file:

```bash
. /root/deploy/hosts.env
N=1
K=<path-to-the-keys-file>
scp "$K" root@$C1:/root/deploy/trustee$N.toml
ssh root@$C1 "k3s kubectl -n step-infra create secret generic trustee$N-keys \
  --from-file=trustee.toml=/root/deploy/trustee$N.toml; \
  shred -u /root/deploy/trustee$N.toml; ls /root/deploy/trustee$N.toml"
```

The last command must answer that the file does not exist. Unmount the
trustee's medium and give it back. Then, on C1, give the instance its sign-in
details and start it:

```bash
. /root/deploy/hosts.env && . /root/deploy/secrets.env
N=1
P=TRUSTEE${N}_PASSWORD
k3s kubectl -n step-infra create secret generic trustee$N-auth \
  --from-env-file=<(printf '%s\n' "TRUSTEE_PASSWORD=${!P}" \
    "TRUSTEE_KEYCLOAK_REALM=$CR" TRUSTEE_KEYCLOAK_CLIENT_ID=api-key-client \
    "TRUSTEE_KEYCLOAK_CLIENT_SECRET=$KC_API_KEY_CLIENT_SECRET")
k3s kubectl -n step-infra rollout restart deployment/trustee$N
k3s kubectl -n step-infra rollout status deployment/trustee$N --timeout=5m
```

The last command must report that the rollout finished. Repeat both blocks
with `N=2` for the second trustee.

Before the key ceremony and before each tally ceremony, check on C1 that both
instances are ready, and confirm it to the election board, which waits for
this confirmation:

```bash
k3s kubectl -n step-infra get deployment trustee1 trustee2    # READY 1/1 for each
```

## Back up the trustee state

The instances keep their protocol state, which they need to continue the
protocol, on C1 under `/var/lib/sequent-data/trustee1` and
`/var/lib/sequent-data/trustee2`.

:::warning
Back up the state only while no ceremony is running, and only encrypted.
Keep the backup and its passphrase in separate locked places.
:::

After the key ceremony, from the administration workstation, with a backup
medium mounted at the path you give:

```bash
. /root/deploy/hosts.env
B=<backup-medium>/trustee-state-$(date +%F).tar.gpg
ssh root@$C1 'k3s kubectl -n step-infra scale --replicas=0 \
  deployment/trustee1 deployment/trustee2'
ssh root@$C1 'tar -C /var/lib/sequent-data -cf - trustee1 trustee2' \
  | gpg --symmetric --cipher-algo AES256 -o "$B"
ssh root@$C1 'k3s kubectl -n step-infra scale --replicas=1 \
  deployment/trustee1 deployment/trustee2'
gpg -d "$B" | tar -tf - | cut -d/ -f1 | sort -u    # trustee1 and trustee2
```

## Remove the trustee keys at the end of the election

When the trustee procedures say so (the retention period has ended and the
election execution data has been deleted), remove the trustees' keys and
state from the servers, on C1:

:::warning
This destroys the trustees' keys files and protocol state on the servers:
the instances can then no longer take part in decrypting the votes. Do it
only at the end of the retention period, with the election board present.
:::

```bash
k3s kubectl -n step-infra scale --replicas=0 deployment/trustee1 deployment/trustee2
k3s kubectl -n step-infra delete secret trustee1-keys trustee2-keys \
  trustee1-auth trustee2-auth
find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 -delete
k3s kubectl -n step-infra get secret trustee1-keys trustee2-keys 2>&1 | grep -c NotFound
find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 | wc -l
```

The last two commands must print `2` and `0`. Then destroy every backup of
the trustee state, and record the removal and the destruction in the election
board's minutes.

## Preparing a trustee workstation

A trustee workstation runs the `trustee` program of the delivered `braid`
image, offline.

:::warning
The workstation has no network connection, an encrypted disk and a screen
lock, and serves only this purpose. Load the trustee tool only from the
verified delivery medium, and only after checking its digest.
:::

The workstation needs podman and skopeo. Mount the delivery medium read-only
with the commands of step 3 of
[Verifying the Delivery](../08-delivery_verification.md), then:

```bash
M=/mnt/temenos-delivery
D=$(sed -n 's/^braid \(sha256:[0-9a-f]*\)$/\1/p' "$M/image-digests.txt")
[ "$(skopeo inspect --format '{{.Digest}}' "oci:$M/images/braid:v10.0.0")" = "$D" ] \
  && echo "braid image verified"
skopeo copy "oci:$M/images/braid:v10.0.0" containers-storage:localhost/braid:v10.0.0
[ "$(podman image inspect --format '{{.Digest}}' localhost/braid:v10.0.0)" = "$D" ] \
  && echo "braid image loaded"
umount "$M" && cryptsetup close temenos-delivery
echo "D=$D" > ~/braid-digest
```

Both checks must print their line; if one does not, do not use the
workstation.

:::warning
Never start the image with its default command on the workstation: that
command starts the trustee server of the platform. Use only the `trustee`
command below. It runs the program `/usr/bin/trustee` instead, gives the
container no network, and mounts only the current directory, which holds the
keys file.
:::

Define the `trustee` command in each new shell:

```bash
. ~/braid-digest
trustee() { podman run --rm --network none --entrypoint /usr/bin/trustee \
  -v "$PWD":/work:Z -w /work "localhost/braid@$D" "$@"; }
trustee --help
```

`trustee --help` lists, among others, the subcommands `generate`,
`public-keys`, `attest` and `verify`. The program sees only the current
directory: run every command from the directory on the trustee's own medium
that holds the keys file, and give file names relative to it, for example
`trustee public-keys --trustee-config trustee.toml`. The trustees use the
subcommands as the trustee procedures of the
[Handbook for Election Organizers (AGD-03)](../../02-election_managers/05-election_organizer_handbook/01-about.md)
describe. The `election-verifier` image is loaded the same way, with its own
name and digest, only where that handbook calls for it.
