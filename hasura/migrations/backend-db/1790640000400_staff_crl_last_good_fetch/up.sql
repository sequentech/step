-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- When a staff revocation list was last downloaded and accepted, apart from
-- the last attempt (`fetched_at`), which may have failed. NULL: never, or
-- before this column existed.
ALTER TABLE sequent_backend.staff_crl ADD COLUMN last_ok_at timestamptz;
