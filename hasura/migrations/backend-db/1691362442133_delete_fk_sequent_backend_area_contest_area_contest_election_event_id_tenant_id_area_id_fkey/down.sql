alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;
