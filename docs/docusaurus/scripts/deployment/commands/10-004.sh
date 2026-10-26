# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl -n step-infra exec deploy/keycloak -- ls /opt/keycloak/providers
k3s kubectl -n step-infra exec deploy/keycloak -- ls /opt/keycloak/providers \
  | awk '/smart-link-hmac/ {n++} END {print n+0}'
