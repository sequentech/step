-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Elections keep the stored default, which reads the same as NULL.

DROP TRIGGER IF EXISTS "default_election_voting_channels"
ON "sequent_backend"."election";

DROP FUNCTION IF EXISTS "sequent_backend"."default_election_voting_channels"();

DROP FUNCTION IF EXISTS "sequent_backend"."election_default_voting_channels"();
