-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT key FROM sequent_backend.lock
WHERE key = $1 AND value = $2 AND expiry_date > clock_timestamp()
FOR UPDATE;
