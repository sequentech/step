-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS "election_seal_guard"
ON "sequent_backend"."election";

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_policy_update"
ON "sequent_backend"."election_event";

DROP TRIGGER IF EXISTS "cast_vote_seal_guard_truncate"
ON "sequent_backend"."cast_vote";

DROP TRIGGER IF EXISTS "cast_vote_seal_guard"
ON "sequent_backend"."cast_vote";

DROP TABLE IF EXISTS "sequent_backend"."ballot_box_seal";

DROP FUNCTION IF EXISTS "sequent_backend"."election_seal_guard"();
DROP FUNCTION IF EXISTS "sequent_backend"."guard_ballot_box_seal_policy_update"();
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_policy"(jsonb);
DROP FUNCTION IF EXISTS "sequent_backend"."cast_vote_seal_guard_truncate"();
DROP FUNCTION IF EXISTS "sequent_backend"."cast_vote_seal_guard"();
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_refuse_truncate"();
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_permanence"();
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_lock_key"(uuid, uuid, uuid, uuid);
