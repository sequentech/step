-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT id, jsonb_build_object('id', id,
    'tenant_id', tenant_id, 'election_event_id', election_event_id,
    'election_id', election_id, 'area_id', area_id, 'created_at', created_at,
    'last_updated_at', last_updated_at, 'annotations', annotations,
    'labels', labels, 'ballot_eml', ballot_eml, 'ballot_signature', ballot_signature,
    'status', status, 'deleted_at', deleted_at) AS data
FROM sequent_backend.ballot_style
WHERE tenant_id = $1 AND election_event_id = $2 AND ballot_publication_id = $3
  AND ($4::uuid IS NULL OR id > $4)
ORDER BY id
LIMIT $5;
