-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Future encrypted ballots skip compression; existing rows are not rewritten.
ALTER TABLE sequent_backend.cast_vote ALTER COLUMN content SET STORAGE EXTERNAL;
