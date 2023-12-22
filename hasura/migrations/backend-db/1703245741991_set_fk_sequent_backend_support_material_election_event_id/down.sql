alter table "sequent_backend"."support_material" drop constraint "support_material_election_event_id_fkey2",
  add constraint "support_material_tenant_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;
