alter table "sequent_backend"."area"
  add constraint "area_election_event_id_tenant_id_parent_id_fkey"
  foreign key ("election_event_id", "tenant_id", "parent_id")
  references "sequent_backend"."area"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
