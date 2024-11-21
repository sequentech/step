alter table "sequent_backend"."results_area_contest" drop constraint "results_area_contest_pkey";
alter table "sequent_backend"."results_area_contest"
    add constraint "results_area_contest_pkey"
    primary key ("id", "tenant_id", "election_event_id", "election_id", "contest_id", "area_id");
