# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
P=/var/lib/sequent-install/provenance.json
APP_VERSION=$(jq -er '.release | select(type == "string")
  | select(test("^[0-9]+[.][0-9]+[.][0-9]+$"))' "$P") &&
APP_HASH=$(jq -er '.images.manifest.stepRevision | select(type == "string")
  | select(test("^[0-9a-f]{40}$"))' "$P") &&
[ -f config/step-env.yaml ] &&
! grep -qE '^  APP_(VERSION|HASH):' config/step-env.yaml &&
printf '  APP_VERSION: "%s"\n  APP_HASH: "%s"\n' "$APP_VERSION" "$APP_HASH" \
  >> config/step-env.yaml &&
grep -E '^  APP_(VERSION|HASH):' config/step-env.yaml
