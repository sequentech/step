-- Checkpoints name their Trellis log by its identity, which a log keeps when it is
-- copied to another database, instead of its key in the single electoral-log
-- database. Moving to a database per election event commits every log again under a
-- new identity, so the checkpoints of the first format describe no stored log.
DELETE FROM "sequent_backend"."electoral_log_checkpoint";
ALTER TABLE "sequent_backend"."electoral_log_checkpoint"
    DROP COLUMN "log_id",
    ADD COLUMN "log_uid" uuid NOT NULL,
    ADD CONSTRAINT "electoral_log_checkpoint_log_uid_tree_size_key"
        UNIQUE ("tenant_id", "election_event_id", "log_uid", "tree_size");
COMMENT ON COLUMN "sequent_backend"."electoral_log_checkpoint"."log_uid" IS
    'Identity of the Trellis log: the event''s board, or a sealed log it continues after an import.';
