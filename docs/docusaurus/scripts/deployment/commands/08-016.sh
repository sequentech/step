# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. ~/braid-digest
trustee() { podman run --rm --network none --entrypoint /usr/bin/trustee \
  -v "$PWD":/work:Z -w /work "localhost/braid@$D" "$@"; }
trustee --help
