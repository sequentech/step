-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

UPDATE sequent_backend.ballot_publication AS publication
SET annotations = COALESCE(annotations, '{}'::jsonb) || jsonb_build_object($6::text, $5::text),
    is_generated = true
WHERE publication.tenant_id = $1
  AND publication.election_event_id = $2
  AND publication.id = $3
  AND publication.deleted_at IS NULL
  AND publication.published_at IS NULL
  AND NOT COALESCE(publication.is_generated, false)
  AND EXISTS (
      SELECT 1 FROM sequent_backend.ballot_publication_snapshot AS snapshot
      WHERE snapshot.tenant_id = publication.tenant_id
        AND snapshot.election_event_id = publication.election_event_id
        AND snapshot.ballot_publication_id = publication.id
        AND snapshot.generation_id = $4
  )
RETURNING publication.id;
