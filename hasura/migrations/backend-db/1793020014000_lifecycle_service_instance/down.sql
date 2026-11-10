-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
ALTER TABLE sequent_backend.self_test_run
    DROP COLUMN repeats,
    DROP COLUMN executing_instance;
DROP TABLE sequent_backend.lifecycle_service_instance;
