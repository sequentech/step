-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT id FROM sequent_backend.election_event
WHERE tenant_id = $1 AND id = $2
FOR NO KEY UPDATE;
