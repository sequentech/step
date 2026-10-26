---
id: hosts_network_time
title: Network, Time and Cluster Checks
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

These checks confirm that the installed servers and their network give the
protection the TOE relies on. Make them before any part of the TOE is
deployed, and stop at the first one that fails.

## Network filtering

:::warning
The site firewall must allow only the connections listed in
[Operational Environment](02-operational_environment.md#network).
:::

First, on C1, list the ports that the cluster's ingress uses (its service
ports and node ports):

<!-- deployment-step: 04-001 -->

**Session: c1.** Run `step 04-001`. Review its implementation with `show 04-001`.

Then, from a computer on a client network (not the administration
workstation), with the variables of `/root/deploy/hosts.env` set and P set to
the port numbers that C1 printed, separated by spaces:

<!-- deployment-step: 04-002 -->

**Session: client.** Run `step 04-002`. Review its implementation with `show 04-002`.

Every line must say `closed`. Port 443 of C1 belongs to the TLS endpoint,
which [Check the TLS profile](05-certificates_tls.md#check-the-tls-profile)
checks. Then check SSH from the separate administration workstation:

<!-- deployment-step: 04-007 -->

**Session: admin.** Run `step 04-007`. Review its implementation with `show 04-007`.

Both server lines must print `open`.

## SSH access

:::warning
Root logs in with a key only. A server that accepts a password is not in the
evaluated configuration.
:::

On each server:

<!-- deployment-step: 04-003 -->

**Session: c1/c2.** Run `step 04-003`. Review its implementation with `show 04-003`.

Option names may be printed with different capitalization; compare the names
without regard to case. The output must show `passwordauthentication no` and
`pubkeyauthentication yes`, and `permitrootlogin` must not be `yes`.

## Authenticated time

:::warning
The servers must take their time only from NTS-authenticated sources.
:::

On each server:

<!-- deployment-step: 04-004 -->

**Session: c1/c2.** Run `step 04-004`. Review its implementation with `show 04-004`.

If a source shows a mode other than NTS, the installed system does not meet
this requirement: stop, and report it to
[security@sequentech.io](mailto:security@sequentech.io). Do not edit the time
configuration of the installed system by hand.

## Encryption of cluster secrets

:::warning
The cluster must store its Secrets encrypted. They hold the database
passwords, the client secrets and the trustee keys.
:::

On C1:

<!-- deployment-step: 04-005 -->

**Session: c1.** Run `step 04-005`. Review its implementation with `show 04-005`.

If the status is not `Enabled`, stop and report it to
[security@sequentech.io](mailto:security@sequentech.io). Do not create any of
the Secrets of the later pages on a cluster that stores them unencrypted.

## Cluster nodes

On C1:

<!-- deployment-step: 04-006 -->

**Session: c1.** Run `step 04-006`. Review its implementation with `show 04-006`.
