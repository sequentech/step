ALTER TABLE "sequent_backend"."electoral_log_checkpoint"
    DROP CONSTRAINT "electoral_log_checkpoint_reason_check",
    ADD CONSTRAINT "electoral_log_checkpoint_reason_check"
        CHECK ("reason" IN ('VOTING_OPENED', 'PERIODIC', 'VOTING_CLOSED', 'TALLY_COMPLETED'));
