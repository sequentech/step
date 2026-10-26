# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
ssh-keygen -lf release/signing-key.pub    # the fingerprint received separately
printf 'sequent-release %s\n' "$(cut -d' ' -f1,2 release/signing-key.pub)" \
  > /tmp/allowed_signers
ssh-keygen -Y verify -f /tmp/allowed_signers -I sequent-release \
  -n sequent-release -s checksums.txt.sig < checksums.txt
sha256sum --check --strict --quiet checksums.txt && echo "all kit files verified"
