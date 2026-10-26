# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
sshd -T | grep -iE '^(passwordauthentication|pubkeyauthentication|permitrootlogin) '
