# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
if grep -nE '^ *(RESULTS_PORTAL_URL|KIOSK_VOTING_PORTAL_URL):' config/step-env.yaml; then
  :
else
  _agd_grep_status=$?
  [ "$_agd_grep_status" -eq 1 ] || return "$_agd_grep_status"
  printf 'Neither excluded-portal setting is present.\n'
fi
