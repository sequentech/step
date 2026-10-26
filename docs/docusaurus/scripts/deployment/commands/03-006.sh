# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
MANIFESTS=$(sed -n 's/^export SEQUENT_MANIFESTS=//p' \
  "$(readlink -f "$(command -v sequent)")")
cp -r "$MANIFESTS" /root/deploy/kubernetes
chmod -R u+w /root/deploy/kubernetes
ls /root/deploy/kubernetes    # namespaces.yaml and one directory per service
