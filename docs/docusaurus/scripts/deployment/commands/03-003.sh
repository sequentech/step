# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
jq . nixos/site.json
./sequent site check --site nixos/site.json --role server && echo "site file valid"
