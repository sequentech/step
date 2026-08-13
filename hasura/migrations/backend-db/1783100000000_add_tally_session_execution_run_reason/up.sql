-- Why a tally session execution row was created. Recorded on the row so that a
-- recount request survives the loss of the celery message that requests it: the
-- task reads the reason from the newest row, and a completed run appends a
-- fresh NORMAL row, so finishing the work is what consumes the reason.
--
-- Nullable with no default so that rows written before this column existed stay
-- distinguishable; windmill reads NULL (and any unrecognised value) as NORMAL.
ALTER TABLE "sequent_backend"."tally_session_execution"
    ADD COLUMN "run_reason" TEXT NULL;

COMMENT ON COLUMN "sequent_backend"."tally_session_execution"."run_reason" IS
    'TallyRunReason: NORMAL | RECOUNT | TIE_BREAK_RERUN. NULL means NORMAL.';
