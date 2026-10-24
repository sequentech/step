---
id: installation_kit
title: Installing the Servers
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The two servers are installed from the installation kit. Sequent builds the
kit for your site from the site file you provide and delivers it on a USB
drive. The fingerprint of the key that signs it comes separately from the
drive. The kit contains the installer image (`nixos/*.iso`) with the operating
system of both servers, the cluster software, the supporting images, the site
file and the cluster's join token; the cluster manifests (`kubernetes/`); the
`sequent` command; and `checksums.txt`, its signature `checksums.txt.sig` and
the public key `release/signing-key.pub`.

The kit is operational environment. The TOE images come from the verified
delivery medium, never from the kit (see [Installing the TOE](08-installing_toe.md)).

## Verify the kit

:::warning
Install nothing from a kit that fails this check. The kit carries the
operating system of the servers and the join token of the cluster.
:::

On the administration workstation, in the kit's top directory, run the kit's
own check with the fingerprint you received separately:

```bash
./sequent verify --fingerprint <signing-key-fingerprint>
```

It must print, in this order, the lines `Signing key fingerprint: ...` (the
fingerprint you received), `Fingerprint matches the expected value.`,
`Signature over checksums.txt is valid.` and
`All N files match checksums.txt. Bundle verified.`

Then check the kit a second time with the standard tools, so that the result
does not depend only on a program from the kit:

```bash
ssh-keygen -lf release/signing-key.pub    # the fingerprint received separately
printf 'sequent-release %s\n' "$(cut -d' ' -f1,2 release/signing-key.pub)" \
  > /tmp/allowed_signers
ssh-keygen -Y verify -f /tmp/allowed_signers -I sequent-release \
  -n sequent-release -s checksums.txt.sig < checksums.txt
sha256sum --check --strict --quiet checksums.txt && echo "all kit files verified"
```

`ssh-keygen -Y verify` must report a good signature for `sequent-release`, and
the last command must print `all kit files verified`.

## Check the site file

The kit's `nixos/site.json` is the site file built into the installer. It
must describe your site exactly:

```bash
jq . nixos/site.json
./sequent site check --site nixos/site.json --role server && echo "site file valid"
```

Compare each value with your network plan: `site.domain`; under `network` the
interface, prefix length, gateway, `dns` and `ntp` (IP addresses of your DNS
and NTS time servers), `podCidr` and `serviceCidr`; under `nodes` the host
names and addresses of C1 (role `server`) and C2 (role `agent`) and any pinned
disks.

:::warning
`admin.authorizedKeys` must contain only the SSH public key of the
administration workstation, and `admin.allowedSourceCidr` only the network of
that workstation. Anyone holding a listed key has root access to both
servers. If a value is wrong, do not install: ask Sequent for a kit built from
a corrected site file.
:::

## Install C1 and C2

:::warning
The installer erases the disk it shows. Keep the USB stick as securely as the
servers: it carries the cluster's join token. After both installations, keep it
locked away or destroy it.
:::

1. Write `nixos/*.iso` to a USB stick and boot C1 from it in UEFI mode.
2. The installer starts by itself. Press **1** for C1.
3. It shows the disk it will erase. Press **Enter** to install; any other key
   cancels without writing anything.
4. When it says *Installation complete*, remove the USB stick and press any
   key. C1 restarts into the installed system.
5. Repeat on C2 with the same stick, pressing **2**. Install C1 first: C2
   joins it at first boot.

If the installer stops, the screen says why and whether anything was written.
Alt+F2 opens a console with the log at `/run/sequent-install.log`.

Check from the administration workstation that both nodes are in the cluster:

```bash
ssh root@<address-of-c1> k3s kubectl get nodes    # c1 and c2, both Ready
```

## Prepare the deployment directory

All later configuration is made in `/root/deploy` on C1, a directory that only
root can read. Log in to C1 and create it with the file of shell variables
that the commands use:

```bash
install -d -m 700 /root/deploy
cat > /root/deploy/hosts.env <<'EOF'
PORTAL_HOST=<voting-portal-host>
ADMIN_HOST=<administration-portal-host>
VERIFIER_HOST=<ballot-verifier-host>
AUTH_HOST=<authentication-server-host>
IDP_ALIAS=<alias-of-the-authentication-server>
C1=<address-of-c1>
C2=<address-of-c2>
R=tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5
export PORTAL_HOST ADMIN_HOST VERIFIER_HOST AUTH_HOST IDP_ALIAS C1 C2 R
EOF
chmod 600 /root/deploy/hosts.env
```

- PORTAL_HOST, ADMIN_HOST and VERIFIER_HOST are the full host names of the
  voting portal, the administration portal and the ballot verifier.
- AUTH_HOST is the host name of the external authentication server.
- IDP_ALIAS is a short name for that server inside the TOE, in lower-case
  letters, digits and hyphens.
- R is the name of the realm of the super-admin tenant, which serves only
  the steps of this guide. Do not change it.
  [Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant)
  later adds CR, the name of the customer's realm.

Copy the file to the same path on the administration workstation, and load
it in every shell, on C1 and on the workstation, with
`. /root/deploy/hosts.env`.

Then make a working copy of the cluster manifests that the `sequent` command
carries. All later changes are made to this copy:

```bash
. /root/deploy/hosts.env
MANIFESTS=$(sed -n 's/^export SEQUENT_MANIFESTS=//p' \
  "$(readlink -f "$(command -v sequent)")")
cp -r "$MANIFESTS" /root/deploy/kubernetes
chmod -R u+w /root/deploy/kubernetes
ls /root/deploy/kubernetes    # namespaces.yaml and one directory per service
```
