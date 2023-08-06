alter table "sequent_backend"."candidate" drop constraint "candidate_pkey";
alter table "sequent_backend"."candidate"
    add constraint "candidate_pkey"
    primary key ("election_event_id", "tenant_id");
