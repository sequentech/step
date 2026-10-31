-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

DROP FUNCTION IF EXISTS "sequent_backend"."vitrine_voting_opened"(uuid, uuid);
DROP FUNCTION IF EXISTS "sequent_backend"."vitrine_status_opened"(jsonb);
DROP TABLE IF EXISTS "sequent_backend"."vitrine_statement";
DROP TABLE IF EXISTS "sequent_backend"."vitrine_event";
DROP TABLE IF EXISTS "sequent_backend"."vitrine_log";
