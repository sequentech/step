alter table "sequent_backend"."results_contest" drop constraint "results_contest_pkey";
alter table "sequent_backend"."results_contest"
    add constraint "results_contest_pkey"
    primary key ("tenant_id", "contest_id", "election_id", "id", "election_event_id");
