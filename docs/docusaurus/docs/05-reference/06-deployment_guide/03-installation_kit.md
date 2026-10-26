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

<!-- deployment-step: 03-001 -->

**Session: admin.** Run `step 03-001`. Review its implementation with `show 03-001`.

It must print, in this order, the lines `Signing key fingerprint: ...` (the
fingerprint you received), `Fingerprint matches the expected value.`,
`Signature over checksums.txt is valid.` and
`All N files match checksums.txt. Bundle verified.`

Then check the kit a second time with the standard tools, so that the result
does not depend only on a program from the kit:

<!-- deployment-step: 03-002 -->

**Session: admin.** Run `step 03-002`. Review its implementation with `show 03-002`.

`ssh-keygen -Y verify` must report a good signature for `sequent-release`, and
the last command must print `all kit files verified`.

## Check the site file

The kit's `nixos/site.json` is the site file built into the installer. It
must describe your site exactly:

<!-- deployment-step: 03-003 -->

**Session: admin.** Run `step 03-003`. Review its implementation with `show 03-003`.

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

<!-- deployment-step: 03-004 -->

**Session: admin.** Run `step 03-004`. Review its implementation with `show 03-004`.

## Prepare the deployment directory

All later configuration is made in `/root/deploy` on C1, a directory that only
root can read. Start the configured C1 session and check the public file of
shell variables created by `./deploy configure c1`:

<!-- deployment-step: 03-005 -->

**Session: c1.** Run `step 03-005`. Review its implementation with `show 03-005`.

- PORTAL_HOST, ADMIN_HOST and VERIFIER_HOST are the full host names of the
  voting portal, the administration portal and the ballot verifier.
- AUTH_HOST is the host name of the external authentication server.
- IDP_ALIAS is a short name for that server inside the TOE, in lower-case
  letters, digits and hyphens.
- R is the name of the realm of the super-admin tenant, which serves only
  the steps of this guide. Do not change it.
  [Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant)
  later adds CR, the name of the customer's realm.

Configure each other machine with the same public site values. Its session
loads its own `hosts.env` automatically. The directory must be mode 700 and the
file mode 600, both owned by root. Keep any later CR addition when reopening a session.

Then make a working copy of the cluster manifests that the `sequent` command
carries. All later changes are made to this copy:

<!-- deployment-step: 03-006 -->

**Session: c1.** Run `step 03-006`. Review its implementation with `show 03-006`.
