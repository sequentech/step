---
id: certificates_tls
title: Certificates and TLS
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This chapter prepares the certificate inputs for
[Install from one configuration file](01-about.md#install-from-one-configuration-file),
and renews the certificate later. The installer checks the inputs and installs the
certificate automatically. You supply the approved files; apart from a change of
the CA file, you do not create cluster Secrets by hand.

## Required TLS profile

Every connection to the TOE's web hosts must use this profile:

| Property | Required value |
| --- | --- |
| Protocol | TLS 1.3 only |
| Cipher suite | TLS_AES_256_GCM_SHA384 only |
| Key exchange group | secp384r1 only |
| Server certificate | ECDSA key on the P-384 curve |
| Names | The voting, administration and ballot-verifier host names in `site.json` |
| Plain HTTP | None; port 80 closed |

The same protocol, cipher and group requirements apply to the external
authentication server ([Connecting the Authentication Server](06-authentication_server.md)).

## Obtain the certificate and its key

**Before installation, on the administration workstation:** obtain an issued
certificate, its matching private key and its CA chain through your approved
certificate procedure. The issuing CA's procedure depends on your organization;
its screens are outside this installer. Request all three web host names and
an ECDSA P-384 key.

The configuration-driven installer reads the key from a protected input file on
the administration workstation and transfers it to C1 over the verified SSH
connection. Keep both machines and these files within the protected
administration environment. Do not paste private-key material into `site.json`,
a terminal, an e-mail or a screenshot.

In your private working configuration, set these file paths:

| `site.json` field | File or value |
| --- | --- |
| `tls.certificate` | PEM server certificate and its intermediate chain |
| `tls.private_key` | Matching private key, owned by root and accessible only to root |
| `tls.ca` | Trusted CA certificate used to validate the chain |
| `tls.ca_sha256` | SHA-256 CA fingerprint received independently, as 64 hexadecimal characters without colons (`openssl x509 -in CA_FILE -noout -fingerprint -sha256` prints it with colons) |

Use absolute paths. Your certificate must be trusted by voters' ordinary,
current browsers. A private CA is not suitable for consumer deployment; adding
such a CA locally does not establish public trust.

**If the certificate is wrong:** obtain the correct certificate for the approved
key and names. A certificate issued for an earlier key does not match a key you
generated later. Resolve any untrusted chain, expired certificate or missing name
before installation.

## Check the certificate

**Where:** a root terminal on the administration workstation, in the deployment
directory. After preparing the complete `site.json` and installing C1 and C2, run:

```bash
./deploy check site.json
```

This is the same preflight command from [Check the site](01-about.md#check-the-site).
It checks the independently supplied CA fingerprint, certificate chain, each
configured host name, P-384 curve and certificate/private-key match. Do not run
it a second time if that preflight has already passed with these exact inputs.

**Expected result:** `PASS: configuration, accepted media and certificate inputs`,
followed by the node checks and `CHECK PASSED — ready to install the platform`.
The private key and client secrets are not printed.

![Actual VM preflight checking certificate inputs and the two installed servers](./assets/installation/installer-preflight.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If it stops:** read the named failure. For a fingerprint mismatch, compare the
independently received record with the supplied file; do not copy an observed
mismatch into the expected field. For a key mismatch, name mismatch, expiry or
untrusted chain, correct the input with the CA. A successful file check does not
replace the live TLS-profile check below.

## Install the certificate

<!-- deployment-step: 05-004 -->

**Automatic part of installation.** After the full preflight passes, the command
below securely transfers the configured certificate and key, creates the TLS
Secret in both cluster namespaces `step-apps` and `step-infra`, and places the
certificate and key of the TLS endpoint on C1:

```bash
./deploy install site.json
```

Run it once from the deployment directory, as described in
[Install the platform](01-about.md#install-the-platform). If that installation
is already complete, do not rerun it for this chapter.

**Expected result:** installation proceeds through its numbered stages and ends
with `INSTALL 4/4: platform installed; customer and trustee commissioning still required`.
The screenshot in [Install the platform](01-about.md#install-the-platform) shows the
complete command and its successful end state.

**If it fails:** keep the error and stage name, and follow
[If something fails](01-about.md#if-something-fails). An existing Secret or occupied
installation may indicate a partial earlier attempt. Do not delete Secrets.

## Set up the TLS endpoint

<!-- deployment-step: 05-005 -->

**Also automatic within `./deploy install site.json`.** The installer places the
certificate, its key, its CA certificate and the three host names in the
protected directory `/var/lib/sequent-site/tls/` on C1. The TLS endpoint,
the TOE part `tls-endpoint`, reads them there: it runs on C1 as the deployment
`tls-endpoint` of the cluster namespace `step-infra`, on C1's own network, and
starts with the other TOE parts at [Deploy](08-installing_toe.md#deploy). It is
the only externally reachable listener on TCP 443. It accepts only the configured
SNI names and the permitted TLS profile, and forwards each request, encrypted again,
through C1's loopback interface to cluster ingress. It forwards nothing it refuses:

- a request whose host name differs from the SNI name gets HTTP `421`;
- a request whose path contains `;`, `#` or `\`, or `%3B`, `%2F` or `%5C` in either
  case, gets HTTP `400` (the query after `?` is not examined);
- a request for the identity service's master realm, `/realms/master` and every
  path under it, gets HTTP `403`.

The TLS endpoint runs only with the memory allocator that its image preloads. If
that allocator is not loaded, the endpoint does not start, and its log
(`./deploy services site.json logs tls-endpoint`) shows
`The hardened_malloc memory allocator is not loaded (LD_PRELOAD=/usr/lib/libhardened_malloc.so); nginx is not started`.
Either the running image or the settings of its deployment then differ from the
accepted installation: run
[Repeat the complete TSF software integrity check](10-verification.md#repeat-the-complete-tsf-software-integrity-check)
and report its result to [security@sequentech.io](mailto:security@sequentech.io)
as a security problem.

The ingress's own listeners must remain unreachable from client networks, and
port 80 must remain closed. [Network filtering](04-hosts_network_time.md#network-filtering)
checks that boundary from a separate client-network machine. Do not expose the
ingress or another application listener to work around a failed TLS endpoint.

The installation command is not a renewal procedure: do not delete the TLS
Secrets or rerun the installation against an occupied cluster. The TLS endpoint
reads new certificate files only when it reloads or restarts, and a restart closes
port 443 until it is ready again. To put a new certificate into service, use
[Renew the certificate](#renew-the-certificate).

## Check the TLS profile

**Where:** the administration workstation, in the deployment directory. After
installation, run:

```bash
./deploy inspect site.json tls
```

This verifies the configured CA fingerprints, tests each of the three TOE host
names from this workstation, and tests the authentication server from **both
C1 and C2**. It uses short, bounded probes and does not change server settings.
It also repeats the allowed connection after the rejection tests: a disconnected
service that refuses everything cannot pass.

![Actual TLS inspection showing verified allowed connections and refused prohibited variants](./assets/installation/inspect-tls.png)

*Terminal capture from a test installation of an earlier build. In the current output the lines of C1 and C2 to the authentication server also name the signature scheme, `ecdsa_secp384r1_sha384`, after `secp384r1`. Your addresses, names and digests differ.*

Lines starting with `TOE` are checks from the administration workstation;
`C1 to authentication server` and `C2 to authentication server` are the checks from
the servers.

**Expected final result:** `TLS PROFILE CHECK PASSED — allowed connections verified and prohibited variants refused`.
All individual lines must pass as summarized below.
[TLS endpoint and ingress ports](10-verification.md#tls-endpoint-and-ingress-ports)
uses this same inspection, so a completed check with unchanged configuration
need not be repeated merely to read another chapter.

| Check | Required result |
| --- | --- |
| Each of the three configured web names | TLS 1.3, `TLS_AES_256_GCM_SHA384`, `secp384r1`, 384-bit server public key, trusted certificate and correct name |
| TLS 1.2, AES-128, ChaCha20 or X25519 | Connection refused |
| Wrong SNI name or no SNI | Connection refused |
| C1 port 80 | Closed |

**If any prohibited connection succeeds or the allowed connection fails:** stop.
Check the configured names, certificate chain and controlled endpoint settings.
Do not disable certificate verification, accept a browser
warning or route clients around the TLS endpoint.

## Renew the certificate

**When:** before the certificate's end date, and before each election event that
ends after that date. Publicly trusted certificates are valid for a few months
only; the duty table of [Operating the servers](11-selftest_handover.md#operating-the-servers)
has the check. A renewed certificate from the same CA file goes into service
without closing port 443: voting can continue, and the election board does not
need to decide. A certificate from another CA file needs
[Change the CA file](#change-the-ca-file) instead.

**Where:** a root terminal on the administration workstation, in the deployment
directory. Mount the kit at `kit.directory` and the delivery medium at
`toe.directory`: as `./deploy check site.json` does, the command first checks the
configuration and every file it names.

To renew the certificate:

1. Obtain the new certificate, its chain and its key as
   [Obtain the certificate and its key](#obtain-the-certificate-and-its-key)
   describes, from the CA of the file that `tls.ca` names. The certificate must
   end after the end of the next election event.
2. Put the certificate with its chain in the file that `tls.certificate` names and
   the key in the file that `tls.private_key` names, owned by root and accessible
   only to root. Leave `tls.ca` and `tls.ca_sha256` as they are.
3. Run the renewal:

   ```bash
   ./deploy renew-tls site.json
   ```

   It prints `Renewing the TLS certificate (valid until …)` with the new
   certificate's end date. If the date is right, type `RENEW TLS CERTIFICATE` when
   prompted. `./deploy` sends the certificate and the key to C1 over the verified
   SSH connection. The TLS endpoint checks and reloads the new files, the cluster
   ingress (Traefik) gets them too, and `./deploy` waits up to 60 seconds until
   both serve the new certificate for all three host names. Only then does it
   replace the installation's own copies on C1.

   **Expected result:** `TLS CERTIFICATE RENEWED: the TLS endpoint and Traefik serve it for every host; run ./deploy inspect site.json tls`.
4. Run the check of [Check the TLS profile](#check-the-tls-profile):

   ```bash
   ./deploy inspect site.json tls
   ```

5. Take a system backup the next time a backup is permitted
   ([Create and verify the encrypted backup](11-selftest_handover.md#create-and-verify-the-encrypted-backup)):
   a backup holds the installation's copies of the certificate and its key.

**If it stops:** each message starts with `Stopped: TLS certificate renewal failed`
and ends with `stop and inspect this stage privately; see If something fails in the Deployment Guide`.
The reason between them says what to do:

| Reason in the message | What to do |
| --- | --- |
| `The CA file that site.json names differs from the installed one; this command renews only a certificate from the same CA file; nothing was changed` | Obtain the certificate from the CA of the installed CA file, or follow [Change the CA file](#change-the-ca-file). |
| `The TLS endpoint did not accept the new certificate files; the previous files are back in place and nothing else was changed` | The current certificate stays in service. Keep the message and follow [If something fails](01-about.md#if-something-fails). |
| `The TLS endpoint or Traefik did not serve the new certificate for every host within 60 seconds; the installation copies are unchanged and the command can be repeated` | Run the command again with the same files. If it stops again, follow [If something fails](01-about.md#if-something-fails). |
| None | Another maintenance operation is running, the services are stopped for maintenance, a trustee backup is running or the TLS endpoint is not ready. Read the state with `./deploy services site.json status`, and run the command again with the same files once that has ended. |

### Change the CA file

The CA file changes when the new certificate comes from another CA, or when the
file that `tls.ca` names gets another root or intermediate certificate.
`./deploy renew-tls site.json` refuses this change; make it with the steps below.

**When:** only on the election board's decision, and never while an election
event is in the execution or evaluation phase. The SHA-256 of the CA file is part
of the approved access-control configuration: after the change, each election
event whose configuration the board approved before fails the self-test check
"Approved access-control configuration" until the board approves a new
configuration version, which it can do only in the preparation phase
([Approving the configuration](../../02-election_managers/05-election_organizer_handbook/06-preparation.md#approving-the-configuration)).
In the execution phase that stops voting, and in the evaluation phase the TOE
refuses the counting. Port 443 stays open throughout.

**Where:** a root terminal on the administration workstation, in the deployment
directory, and from step 3 a root shell on C1.

To change the CA file:

1. On the workstation, put the new certificate with its chain, its key and the new
   CA certificate in the files that `tls.certificate`, `tls.private_key` and
   `tls.ca` name, and the new CA fingerprint, received independently, in
   `tls.ca_sha256` ([Obtain the certificate and its key](#obtain-the-certificate-and-its-key)).
2. Copy the three files to the new directory `/root/tls-new` on C1. The key travels
   on the SSH connection's standard input, never on a command line. If `mkdir`
   reports that the directory exists, an earlier attempt left it: remove it on C1
   with `rm -r /root/tls-new` and repeat this step.

   ```bash
   SERVER=$(jq -er '.nodes.c1' site.json)
   ADMIN_KEY=$(jq -er '.ssh_identity' site.json)
   ssh -i "$ADMIN_KEY" "root@$SERVER" 'umask 077; mkdir /root/tls-new'
   ssh -i "$ADMIN_KEY" "root@$SERVER" 'umask 077; cat > /root/tls-new/tls.key' < "$(jq -er '.tls.private_key' site.json)"
   ssh -i "$ADMIN_KEY" "root@$SERVER" 'umask 077; cat > /root/tls-new/tls.crt' < "$(jq -er '.tls.certificate' site.json)"
   ssh -i "$ADMIN_KEY" "root@$SERVER" 'umask 077; cat > /root/tls-new/ca.pem' < "$(jq -er '.tls.ca' site.json)"
   ```

3. Open a root shell on C1 and change to the TLS endpoint's directory. Steps 4 to 8
   run in that shell:

   ```bash
   ssh -i "$ADMIN_KEY" "root@$SERVER"
   cd /var/lib/sequent-site/tls
   ```

4. Have the TLS endpoint trust both the current and the new CA, then reload it:

   ```bash
   cat /root/tls-new/ca.pem ca.pem > .ca.new && chmod 644 .ca.new && mv -f .ca.new ca.pem
   k3s kubectl -n step-infra exec deploy/tls-endpoint -- nginx -t -q \
     && k3s kubectl -n step-infra exec deploy/tls-endpoint -- nginx -s reload
   ```

5. Replace the certificate Secret in both cluster namespaces; never delete it:

   ```bash
   for ns in step-apps step-infra; do
     k3s kubectl -n "$ns" create secret tls step-tls-cert --cert=/root/tls-new/tls.crt \
       --key=/root/tls-new/tls.key --dry-run=client -o yaml | k3s kubectl -n "$ns" replace -f -
   done
   ```

   **Expected result:** `secret/step-tls-cert replaced`, twice.
6. Install the new certificate, its key and the new CA file alone for the TLS
   endpoint, then reload it again:

   ```bash
   install -m 644 /root/tls-new/tls.crt .tls.crt.new
   install -m 600 /root/tls-new/tls.key .tls.key.new
   install -m 644 /root/tls-new/ca.pem .ca.new
   mv -f .tls.key.new tls.key && mv -f .tls.crt.new tls.crt && mv -f .ca.new ca.pem
   k3s kubectl -n step-infra exec deploy/tls-endpoint -- nginx -t -q \
     && k3s kubectl -n step-infra exec deploy/tls-endpoint -- nginx -s reload
   ```

7. Replace the CA file of the self-tests:

   ```bash
   k3s kubectl -n step-apps create configmap self-test-tls-ca --from-file=ca.pem=/root/tls-new/ca.pem \
     --dry-run=client -o yaml | k3s kubectl -n step-apps replace -f -
   ```

   **Expected result:** `configmap/self-test-tls-ca replaced`.
8. Replace the installation's copies, remove the transfer directory and close the
   shell:

   ```bash
   for f in tls.crt tls.key ca.pem; do
     install -m 600 "/root/tls-new/$f" "/root/deploy/tls/.$f.new"
     mv -f "/root/deploy/tls/.$f.new" "/root/deploy/tls/$f"
   done
   rm -r /root/tls-new
   exit
   ```

9. On the workstation, check the TLS profile and the platform:

   ```bash
   ./deploy inspect site.json tls
   ./deploy verify site.json
   ```

   **Expected result:** `TLS PROFILE CHECK PASSED — allowed connections verified and prohibited variants refused`,
   and at the end of the second command
   `PASS: installed platform readiness, restricted services, secret store and HTTPS discovery`.
10. Take a system backup
    ([Create and verify the encrypted backup](11-selftest_handover.md#create-and-verify-the-encrypted-backup)).
11. Tell the administrators and the election board that the change is complete. A
    self-test run made during the change can have failed the check "Trusted HTTPS
    channels". The board approves a new configuration version of each election
    event whose configuration it approved before the change.

If a `nginx -t -q` reports an error, the TLS endpoint keeps serving what it loaded
before. Do not reload or restart it; keep the output and follow
[If something fails](01-about.md#if-something-fails). If the endpoint cannot be
reloaded, for example because its pod is not running,
`./deploy services site.json restart tls-endpoint` also makes it read the files,
but it closes port 443 until the endpoint is ready again
([Restart an authorized failed component](11-selftest_handover.md#restart-an-authorized-failed-component)).
