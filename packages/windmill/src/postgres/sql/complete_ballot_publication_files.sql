-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

UPDATE sequent_backend.ballot_publication AS publication
SET annotations = COALESCE(annotations, '{}'::jsonb) || jsonb_build_object($5::text, $4::text),
    is_generated = true
WHERE publication.tenant_id = $1
  AND publication.election_event_id = $2
  AND publication.id = $3
  AND publication.deleted_at IS NULL
  AND publication.published_at IS NULL
  AND NOT COALESCE(publication.is_generated, false)
RETURNING publication.id;
