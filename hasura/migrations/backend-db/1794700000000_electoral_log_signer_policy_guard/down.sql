-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS electoral_log_signer_policy_guard ON sequent_backend.election_event;
DROP FUNCTION IF EXISTS sequent_backend.guard_electoral_log_signer_policy();
DROP FUNCTION IF EXISTS sequent_backend.electoral_log_signer_policy_of(jsonb);
