-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Events still waiting here are lost with the table: publish them first by
-- letting relay_electoral_log_outbox drain it.
DROP TABLE IF EXISTS sequent_backend.electoral_log_outbox;
