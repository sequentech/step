-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT id, ballot_eml
FROM sequent_backend.ballot_style
WHERE tenant_id = $1 AND election_event_id = $2 AND ballot_publication_id = $3
  AND ($4::uuid IS NULL OR election_id = $4)
  AND ($5::uuid IS NULL OR id > $5)
ORDER BY id
LIMIT $6;
