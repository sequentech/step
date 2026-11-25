-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- An election event without a label stays open to every admin user of the
-- tenant; a labelled one only to admin users whose permission labels hold it.
ALTER TABLE sequent_backend.election_event
    ADD COLUMN IF NOT EXISTS permission_label text;
