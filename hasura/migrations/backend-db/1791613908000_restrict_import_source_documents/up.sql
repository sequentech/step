-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

UPDATE sequent_backend.document AS document
SET annotations = jsonb_set(
    CASE
        WHEN jsonb_typeof(document.annotations) = 'object' THEN document.annotations
        ELSE '{}'::jsonb
    END,
    '{access}',
    CASE
        WHEN jsonb_typeof(document.annotations -> 'access') = 'object'
            THEN document.annotations -> 'access'
        ELSE '{}'::jsonb
    END || '{"voter_secret_attributes": true}'::jsonb
)
FROM sequent_backend.tasks_execution AS task
WHERE task.type IN ('IMPORT_USERS', 'IMPORT_ELECTION_EVENT')
    AND task.tenant_id = document.tenant_id
    AND task.annotations ->> 'document_id' = document.id::text;
