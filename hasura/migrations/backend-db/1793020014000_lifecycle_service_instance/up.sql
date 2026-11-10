-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The lease stays one row per component. These rows tell a component's processes
-- apart: one stopping or starting changes nothing while another is working.
CREATE TABLE sequent_backend.lifecycle_service_instance (
    component text NOT NULL CHECK (component IN ('harvest','windmill')),
    instance uuid NOT NULL,
    started_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    renewed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (component,instance)
);

-- The process executing a run, and how many times a run was repeated because its
-- process ended before finishing it.
ALTER TABLE sequent_backend.self_test_run
    ADD COLUMN executing_instance uuid,
    ADD COLUMN repeats smallint NOT NULL DEFAULT 0;
