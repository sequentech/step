#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Copies one built image from a local OCI layout to several repositories, and
# fails unless each of them ends up serving the digest that was built.
#
# Usage: push_image.sh <oci-layout-dir> "<tag> [<tag>...]" <repository>...
#
# A repository is a containers-transports(5) reference without the tag, e.g.
# docker://europe-west4-docker.pkg.dev/my-project/my-repo/my-image.

set -euo pipefail

if [ "$#" -lt 3 ]; then
  echo "usage: $0 <oci-layout-dir> \"<tag> [<tag>...]\" <repository>..." >&2
  exit 2
fi

layout=$1
read -r -a tags <<<"$2"
shift 2

manifest_digest() {
  skopeo inspect --raw "$1" | sha256sum | cut -d' ' -f1
}

source_ref="oci:$layout"
built=$(manifest_digest "$source_ref")
echo "Built sha256:$built"

for repository in "$@"; do
  for tag in "${tags[@]}"; do
    destination="$repository:$tag"
    # --all keeps the index and its attestation manifests, which are part of the
    # digest. --preserve-digests fails rather than rewriting anything.
    skopeo copy --all --preserve-digests --retry-times 3 "$source_ref" "$destination"

    pushed=$(manifest_digest "$destination")
    if [ "$pushed" != "$built" ]; then
      echo "$destination serves sha256:$pushed, expected sha256:$built" >&2
      exit 1
    fi
    echo "$destination@sha256:$pushed"
  done
done
