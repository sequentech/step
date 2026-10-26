# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# configure already created the one public site configuration; never overwrite CR.
test -r /root/deploy/hosts.env
. /root/deploy/hosts.env
test -n "$PORTAL_HOST" && test -n "$ADMIN_HOST" && test -n "$VERIFIER_HOST"
test -n "$AUTH_HOST" && test -n "$IDP_ALIAS" && test -n "$C1" && test -n "$C2"
test "$R" = tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5
stat -c '%a %U %n' /root/deploy /root/deploy/hosts.env
