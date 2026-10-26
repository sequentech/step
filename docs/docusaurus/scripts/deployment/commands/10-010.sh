# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
(
for app in harvest windmill; do
  k3s kubectl -n step-apps exec "deploy/$app" -c "$app" -- sh -eu -c '
    test "${SECRETS_BACKEND:-}" = EnvVarMasterSecret
    test "${#MASTER_SECRET}" -eq 64
    printf "Secret-store configuration: PASS\n"
  ' || exit 1
done
)
