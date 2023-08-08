alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey",
  add constraint "area_contest_tenant_id_election_event_id_area_id_fkey"
  foreign key ("tenant_id", "election_event_id", "area_id")
  references "sequent_backend"."area"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;
