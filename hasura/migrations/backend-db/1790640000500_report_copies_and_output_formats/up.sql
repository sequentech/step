-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- How many copies a report prints and the formats it is generated in, as the
-- signed configuration sets them. NULL: one copy, the type's default format.
ALTER TABLE sequent_backend.report
    ADD COLUMN copies integer CHECK (copies IS NULL OR copies > 0),
    ADD COLUMN output_formats text[];
