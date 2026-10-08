DELETE FROM "sequent_backend"."electoral_log_checkpoint";
ALTER TABLE "sequent_backend"."electoral_log_checkpoint"
    DROP CONSTRAINT "electoral_log_checkpoint_log_uid_tree_size_key",
    DROP COLUMN "log_uid",
    ADD COLUMN "log_id" bigint NOT NULL,
    ADD UNIQUE ("tenant_id", "election_event_id", "log_id", "tree_size");
