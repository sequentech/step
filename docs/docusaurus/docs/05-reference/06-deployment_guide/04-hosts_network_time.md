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

```bash
k3s kubectl -n kube-system get service traefik \
  -o jsonpath='{.spec.ports[*].port} {.spec.ports[*].nodePort}{"\n"}'
```

Then, from a computer on a client network (not the administration
workstation), with the variables of `/root/deploy/hosts.env` set and P set to
the port numbers that C1 printed, separated by spaces:

```bash
P="<ports-printed-on-c1>"
port() { timeout 5 bash -c "</dev/tcp/$1/$2" 2>/dev/null && echo "$1:$2 open" \
  || echo "$1:$2 closed"; }
for p in 22 80 6443 10250 $P; do [ $p = 443 ] || port "$C1" $p; done  # all closed
for p in 22 80 443 6443 10250 $P; do port "$C2" $p; done   # all closed
```

Every line must say `closed`. Port 443 of C1 belongs to the TLS endpoint,
which [Check the TLS profile](05-certificates_tls.md#check-the-tls-profile)
checks. From the administration workstation, `port "$C1" 22` and
`port "$C2" 22` must print `open`.

## SSH access

:::warning
Root logs in with a key only. A server that accepts a password is not in the
evaluated configuration.
:::

On each server:

```bash
sshd -T | grep -iE '^(passwordauthentication|pubkeyauthentication|permitrootlogin) '
```

Option names may be printed with different capitalization; compare the names
without regard to case. The output must show `passwordauthentication no` and
`pubkeyauthentication yes`, and `permitrootlogin` must not be `yes`.

## Authenticated time

:::warning
The servers must take their time only from NTS-authenticated sources.
:::

On each server:

```bash
chronyc -N authdata    # every source: NTS in the Mode column
chronyc tracking       # "Leap status" is Normal
chronyc sources        # the selected source (marked *) is one of the site's servers
```

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

```bash
k3s secrets-encrypt status    # Encryption Status: Enabled
```

If the status is not `Enabled`, stop and report it to
[security@sequentech.io](mailto:security@sequentech.io). Do not create any of
the Secrets of the later pages on a cluster that stores them unencrypted.

## Cluster nodes

On C1:

```bash
k3s kubectl get nodes -o wide    # c1 and c2 Ready, at the addresses of the site file
k3s kubectl get pods -A          # only kube-system pods, all Running or Completed
```
