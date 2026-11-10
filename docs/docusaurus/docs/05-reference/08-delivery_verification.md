---
id: delivery_verification
title: Verifying the Delivery
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Carry out these checks before deploying anything. They are the verification
steps of the signed delivery note, in the same order, with the commands for
each; where this chapter and the note differ, the note applies. If a check fails,
stop: do not deploy, and report it to
[security@sequentech.io](mailto:security@sequentech.io). The delivery is then
repeated.

This chapter is the first procedure of the Deployment Guide. Nothing is installed
or deployed before [Confirm receipt](#7-confirm-receipt) is complete.

The commands run in bash on Linux and need curl, GnuPG, cryptsetup, skopeo and
the GNU coreutils, diffutils and findutils. Save the signed delivery note as
`delivery-note.asc` in the current directory, and in the commands replace
`/dev/sdX` with the device of the delivery medium (see `lsblk`) and `v10.0.0`
with the release tag stated in the note.

Run the commands on the administration workstation described in
[Operational Environment](06-deployment_guide/02-operational_environment.md).

## Verify the signature on the delivery note {#1-verify-the-signature-on-the-delivery-note}

Verify it against the fingerprint of the Sequent security-contact key that you
hold independently of this delivery: from the published
[Vulnerability Disclosure Policy](https://docs.sequentech.io/docusaurus/main/docs/technology/vulnerability-disclosure-policy)
and the keyserver record it names, from the registration of your contact, or
from this guide as received with an earlier delivery. The fingerprint is
`DB6B 035E 872A B5DA 90C7  194A 60D3 CBE3 0BAC 91ED`.

```bash
curl --fail --output pgp-key.txt \
  https://keys.openpgp.org/vks/v1/by-fingerprint/DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED
gpg --show-keys pgp-key.txt      # the fingerprint must be the one you hold
gpg --import pgp-key.txt
# the next command must report "Good signature" made with the key of that fingerprint
gpg --verify delivery-note.asc
gpg --output delivery-note.txt --decrypt delivery-note.asc   # the signed text, used in section 2.5
```

Do not act on a note that is unsigned or whose signature does not verify.

## Inspect the shipment {#2-inspect-the-shipment}

Report any damage to the shipment, or any discrepancy between what arrived and
the delivery note, before going on.

The delivery consists of the signed delivery note, the encrypted delivery
medium and its passphrase, which comes separately from the medium. The medium
holds the eleven TOE parts as OCI images (harvest, windmill, braid, b4,
ballot-verifier, election-verifier, keycloak, tls-endpoint, graphql-engine,
voting-portal and admin-portal), the guidance documents, and the files
`checksums.txt` and `image-digests.txt`.
[Verify the files and the images](#4-verify-the-files-and-the-images) and
[Check the guidance version](#6-check-the-guidance-version) check its contents.

## Decrypt the medium {#3-decrypt-the-medium}

Use the passphrase received separately, never one that came with the package.
A medium that does not decrypt with it is a failed delivery. Sequent gives the
delivery note and, by another channel, the passphrase to your organization's
registered contact, who gives both to the operator. Keep the passphrase in a locked
place separate from the medium: it is needed each time the medium is mounted,
including when a trustee workstation is prepared
([Set up the public configuration](06-deployment_guide/08-installing_toe.md#set-up-the-public-configuration)).

```bash
sudo -s                                                 # sections 2.3 to 2.6 run as root
cryptsetup open --readonly /dev/sdX temenos-delivery    # asks for the passphrase
mkdir -p /mnt/temenos-delivery
mount -o ro /dev/mapper/temenos-delivery /mnt/temenos-delivery
M=/mnt/temenos-delivery
```

## Verify the files and the images {#4-verify-the-files-and-the-images}

Every file must match `checksums.txt`, none may be missing or unlisted, and
each image must have the digest that `image-digests.txt` gives for it.

```bash
(cd "$M" && sha256sum --check --strict --quiet checksums.txt) \
  && echo "all listed files verified"
diff <(cut -c67- "$M/checksums.txt" | sort) \
  <(find "$M" -type f ! -path "$M/checksums.txt" -printf '%P\n' | sort) \
  && echo "no unlisted files"
grep ' sha256:' "$M/image-digests.txt" | while read -r name digest; do
  [ "$(skopeo inspect --format '{{.Digest}}' "oci:$M/images/$name:v10.0.0")" = "$digest" ] \
    && echo "$name verified" || echo "$name FAILED"
done
```

The medium must hold exactly the eleven TOE parts, each verified by the loop
above:

```bash
ls -A "$M"          # checksums.txt guidance image-digests.txt images
                    # (an empty lost+found directory may also be listed)
ls "$M/images"      # admin-portal b4 ballot-verifier braid election-verifier
                    # graphql-engine harvest keycloak tls-endpoint voting-portal windmill
grep -c ' sha256:' "$M/image-digests.txt"    # 11
```

## Compare with the signed delivery note {#5-compare-with-the-signed-delivery-note}

The image digests and the SHA-256 of `checksums.txt`, the package hash, must be
the ones stated in the signed note. This is what distinguishes an authentic
delivery from a counterfeit one.

```bash
sed -n 's/^- \([a-z0-9-]*\): `\(sha256:[0-9a-f]*\)`$/\1 \2/p' delivery-note.txt | sort \
  | diff - <(grep ' sha256:' "$M/image-digests.txt" | sort) \
  && echo "image digests match the note"
grep -qF "$(sha256sum < "$M/checksums.txt" | cut -c1-64)" delivery-note.txt \
  && echo "package hash matches the note"
```

The header of `image-digests.txt` must name the TOE reference and the release
tag that the note states:

```bash
grep -E '^(toe_reference|tag): ' "$M/image-digests.txt"
grep -E '(TOE reference|Release tag):' delivery-note.txt
# both: Sequent Voting Platform - uniWAHL Version, v10.0.0 and the same release tag
```

Keep the SHA-256 of `image-digests.txt` with the delivery note: it goes to
`toe.manifest_sha256` in `site.json`, and the administrators compare it with the
value "SHA-256 of the accepted digest list" that the admin portal shows
([Verify the TOE software](../02-election_managers/04-administrator_handbook/07-self_tests.md#verify-the-toe-software)).
Read it with this command:

```bash
sha256sum "$M/image-digests.txt"
```

## Check the guidance version {#6-check-the-guidance-version}

The guidance under `guidance/` on the medium must carry the release tag of the
delivered software. Each handbook states it on its first page, and the guidance
manifest records it.

```bash
grep '"tag"' "$M/guidance/manifest.json"    # must be the release tag in the note
```

The guidance consists of the four handbooks, their index and the directory
`deployment` of the installer, and the manifest names the TOE reference:

```bash
ls "$M/guidance"    # AGD-01.pdf AGD-02.pdf AGD-03.pdf AGD-04.pdf deployment index.pdf manifest.json
grep '"toe_reference"' "$M/guidance/manifest.json"    # the TOE reference in the note
```

Then close the medium, and keep it with the delivery note in a locked place.
The later procedures mount it again, read-only, with the commands of
[Decrypt the medium](#3-decrypt-the-medium).

```bash
umount "$M"
cryptsetup close temenos-delivery
```

## Confirm receipt {#7-confirm-receipt}

Confirm receipt to the Sequent release manager through the contact channel of
your delivery, stating the TOE reference received and that verification succeeded.
This confirmation completes the delivery.

Send the confirmation within the period that the delivery note states. Then
continue with
[Operational Environment](06-deployment_guide/02-operational_environment.md).
