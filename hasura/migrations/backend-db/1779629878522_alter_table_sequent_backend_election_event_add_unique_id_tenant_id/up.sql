alter table "sequent_backend"."election_event"
    add constraint "election_event_id_tenant_id_key" unique ("tenant_id", "id");
