---
id: delivery_verification
title: Verifying the Delivery
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

> Scope: These checks apply to an evaluated uniWAHL delivery package supplied with
> a signed delivery note and the artifacts named below. They do not imply that
> every build of the main branch produces such a package.

Carry out these checks before deploying anything. They are the verification
steps of the signed delivery note, in the same order, with the commands for
each; where this page and the note differ, the note applies. If a check fails,
stop: do not deploy, and report it to
[security@sequentech.io](mailto:security@sequentech.io). The delivery is then
repeated.

The commands run in bash on Linux and need curl, GnuPG, cryptsetup, skopeo, Python 3 and
the GNU coreutils, diffutils and findutils. Save the signed delivery note as
`delivery-note.asc` in the current directory, and in the commands replace
`/dev/sdX` with the device of the delivery medium (see `lsblk`) and `v10.0.0`
with the release tag stated in the note.

## 1. Verify the signature on the delivery note

Verify it against the fingerprint of the Sequent security-contact key that you
hold independently of this delivery: from the published
[Vulnerability Disclosure Policy](https://docs.sequentech.io/docusaurus/main/docs/technology/vulnerability-disclosure-policy)
and the keyserver record it names, from the registration of your contact, or
from this guide as received with an earlier delivery. The fingerprint is
`DB6B 035E 872A B5DA 90C7  194A 60D3 CBE3 0BAC 91ED`.

```bash
curl --fail --output pgp-key.txt https://keys.openpgp.org/vks/v1/by-fingerprint/DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED
gpg --show-keys pgp-key.txt      # the fingerprint must be the one you hold
gpg --import pgp-key.txt
gpg --verify delivery-note.asc   # must report "Good signature" made with the key of that fingerprint
gpg --output delivery-note.txt --decrypt delivery-note.asc   # the signed text, used in step 5
```

Do not act on a note that is unsigned or whose signature does not verify.

## 2. Inspect the shipment

Report any damage to the shipment, or any discrepancy between what arrived and
the delivery note, before going on.

## 3. Decrypt the medium

Use the passphrase received separately, never one that came with the package.
A medium that does not decrypt with it is a failed delivery.

```bash
sudo bash                                               # steps 3 to 6 run in Bash as root
cryptsetup open --readonly /dev/sdX temenos-delivery    # asks for the passphrase
mkdir -p /mnt/temenos-delivery
mount -o ro /dev/mapper/temenos-delivery /mnt/temenos-delivery
M=/mnt/temenos-delivery
DELIVERY_TAG='v10.0.0'    # copy the exact release tag from the verified delivery note
```

## 4. Verify the files and the images

Every file must match `checksums.txt`, none may be missing or unlisted, and
each image must have the digest that `image-digests.txt` gives for it. The
manifest has one `image-name sha256:<64-lowercase-hex-digits>` entry per image;
blank lines and lines beginning with `#` are ignored. The block exits nonzero
on any failed check, malformed entry or empty manifest. Stop if it fails.

```bash
(
  set -euo pipefail
  (cd "$M" && sha256sum --check --strict --quiet checksums.txt)
  echo "all listed files verified"
  diff <(cut -c67- "$M/checksums.txt" | sort) <(find "$M" -type f ! -path "$M/checksums.txt" -printf '%P\n' | sort)
  echo "no unlisted files"

  failed=0
  count=0
  while read -r name digest extra || [[ -n "$name$digest$extra" ]]; do
    [[ -z "$name" || "$name" == \#* ]] && continue
    count=$((count + 1))
    if [[ ! "$name" =~ ^[a-z0-9][a-z0-9-]*$ || ! "$digest" =~ ^sha256:[0-9a-f]{64}$ || -n "$extra" ]]; then
      echo "Malformed image digest entry: $name" >&2
      failed=1
    elif actual=$(skopeo inspect --format '{{.Digest}}' "oci:$M/images/$name:$DELIVERY_TAG") && [[ "$actual" == "$digest" ]]; then
      echo "$name verified"
    else
      echo "$name FAILED" >&2
      failed=1
    fi
  done < "$M/image-digests.txt"
  [[ "$count" -gt 0 && "$failed" -eq 0 ]] || exit 1
)
```

## 5. Compare with the signed delivery note

The image digests and the SHA-256 of `checksums.txt`, the package hash, must be
the ones stated in the signed note. This is what distinguishes an authentic
delivery from a counterfeit one.

```bash
sed -n 's/^- \([a-z0-9-]*\): `\(sha256:[0-9a-f]*\)`$/\1 \2/p' delivery-note.txt | sort | diff - <(grep ' sha256:' "$M/image-digests.txt" | sort) && echo "image digests match the note"
grep -qF "$(sha256sum < "$M/checksums.txt" | cut -c1-64)" delivery-note.txt && echo "package hash matches the note"
```

## 6. Check the guidance version

The guidance under `guidance/` on the medium must carry the release tag of the
delivered software. Each handbook states it on its first page, and the guidance
manifest records it.

```bash
python3 - "$M/guidance/manifest.json" "$DELIVERY_TAG" <<'PYTHON'
import json
import sys

def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate manifest field: " + key)
        result[key] = value
    return result

with open(sys.argv[1]) as source:
    manifest = json.load(source, object_pairs_hook=unique_fields)
expected = sys.argv[2]
if not expected or not isinstance(manifest, dict) or manifest.get("tag") != expected:
    sys.exit("Guidance tag missing or different from the verified delivery note; STOP")
print("guidance tag matches the verified delivery note")
PYTHON
```

## 7. Confirm receipt

Confirm receipt to the Sequent release manager, stating the TOE reference
received and that verification succeeded. This confirmation completes the
delivery.
