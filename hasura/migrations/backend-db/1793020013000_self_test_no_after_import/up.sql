-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
UPDATE sequent_backend.self_test_settings
SET settings = jsonb_set(
    settings,
    '{additional_conditions}',
    (settings -> 'additional_conditions') - 'after-import'
)
WHERE settings -> 'additional_conditions' ? 'after-import';

-- The stale-run sweep reads unfinished runs by their typed trigger, so none may
-- remain with the removed one. Finished runs stay as history.
UPDATE sequent_backend.self_test_run
SET finished_at = now(),
    outcome = 'recoverable-failure',
    result = result || jsonb_build_object(
        'finished_at', now(),
        'outcome', 'recoverable-failure'
    )
WHERE trigger = 'after-import' AND finished_at IS NULL;
