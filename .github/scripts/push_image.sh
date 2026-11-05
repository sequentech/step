#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Copies one built image from a local OCI layout to several destinations, and
# fails unless each of them ends up serving the digest that was built.
#
# Usage: push_image.sh <oci-layout-dir> <destination>... [--immutable <destination>...]
#
# A destination is a containers-transports(5) reference with its tag, e.g.
# docker://europe-west4-docker.pkg.dev/my-project/my-repo/my-image:my-tag.
#
# Destinations after --immutable are in a registry that refuses to move a tag.
# If one already serves another digest, nothing is copied anywhere, so a rebuild
# can't leave the registries serving different images under one tag. They are
# copied last, so a failure elsewhere leaves them free for a re-run.

set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <oci-layout-dir> <destination>... [--immutable <destination>...]" >&2
  exit 2
fi

manifest_digest() {
  skopeo inspect --raw "$1" | sha256sum | cut -d' ' -f1
}

source_ref="oci:$1"
shift
built=$(manifest_digest "$source_ref")
echo "Built sha256:$built"

destinations=()
immutable=false
for argument in "$@"; do
  if [ "$argument" = "--immutable" ]; then
    immutable=true
    continue
  fi
  destinations+=("$argument")
  if "$immutable" && existing=$(manifest_digest "$argument" 2>/dev/null) && [ "$existing" != "$built" ]; then
    echo "$argument is immutable and already serves sha256:$existing, not the sha256:$built just built. Nothing was pushed. Publish this build under a new tag." >&2
    exit 1
  fi
done

for destination in "${destinations[@]}"; do
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
