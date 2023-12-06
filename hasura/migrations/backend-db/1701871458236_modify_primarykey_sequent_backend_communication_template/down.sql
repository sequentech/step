alter table "sequent_backend"."communication_template" drop constraint "communication_template_pkey";
alter table "sequent_backend"."communication_template"
    add constraint "communication_template_pkey"
    primary key ("id", "election_event_id", "tenant_id");
