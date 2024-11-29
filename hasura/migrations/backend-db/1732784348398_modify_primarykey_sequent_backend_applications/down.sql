? alter table "sequent_backend"."applications" drop constraint "applications_pkey";
alter table "sequent_backend"."applications"
    add constraint "applications_pkey"
    primary key ("id", "tenant_id", "election_event_id");
