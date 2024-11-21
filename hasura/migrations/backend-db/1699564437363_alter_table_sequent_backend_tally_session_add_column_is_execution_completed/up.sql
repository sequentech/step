alter table "sequent_backend"."tally_session" add column "is_execution_completed" boolean
 not null default 'false';
