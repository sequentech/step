-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS "set_sequent_backend_tally_session_last_updated_at"
ON "sequent_backend"."tally_session";
