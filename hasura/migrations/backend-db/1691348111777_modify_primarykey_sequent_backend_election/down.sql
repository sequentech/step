alter table "sequent_backend"."election" drop constraint "election_pkey";
alter table "sequent_backend"."election"
    add constraint "election_pkey"
    primary key ("tenant_id", "id", "election_event_id");
