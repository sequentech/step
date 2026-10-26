# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
grep -rl -e portal.local -e admin.local -e verifier.local . | xargs sed -i \
  -e "s/portal\.local/$PORTAL_HOST/g" -e "s/admin\.local/$ADMIN_HOST/g" \
  -e "s/verifier\.local/$VERIFIER_HOST/g"
{ grep -rn '\.local\b' . || test "$?" = 1; } | { grep -v -e sequentech.local -e svc.cluster.local || test "$?" = 1; }
