alter table "sequent_backend"."results_election" drop constraint "results_election_pkey";
alter table "sequent_backend"."results_election"
    add constraint "results_election_pkey"
    primary key ("election_event_id", "election_id", "results_event_id", "id", "tenant_id");
