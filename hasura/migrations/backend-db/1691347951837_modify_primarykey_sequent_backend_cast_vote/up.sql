BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."cast_vote" DROP CONSTRAINT "cast_vote_pkey";

ALTER TABLE "sequent_backend"."cast_vote"
    ADD CONSTRAINT "cast_vote_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
