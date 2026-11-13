-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- An unset voting_channels column enables online voting only, as
-- VotingChannels::default() reads it when a vote is cast. The election form
-- fills an unset value from the tenant's channels instead, so its next save
-- switches channels on, which guard_election_server_state refuses once voting
-- has started. Store the default instead of NULL, for existing elections and
-- for inserts and imports that leave it unset.

CREATE OR REPLACE FUNCTION "sequent_backend"."election_default_voting_channels"()
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT '{"online": true, "kiosk": null, "telephone": null, "paper": null, "early_voting": null}'::jsonb;
$$;

CREATE OR REPLACE FUNCTION "sequent_backend"."default_election_voting_channels"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.voting_channels := COALESCE(
        NEW.voting_channels,
        "sequent_backend"."election_default_voting_channels"()
    );
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "default_election_voting_channels"
ON "sequent_backend"."election";

CREATE TRIGGER "default_election_voting_channels"
BEFORE INSERT OR UPDATE OF voting_channels
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."default_election_voting_channels"();

UPDATE "sequent_backend"."election"
SET voting_channels = "sequent_backend"."election_default_voting_channels"()
WHERE voting_channels IS NULL;
