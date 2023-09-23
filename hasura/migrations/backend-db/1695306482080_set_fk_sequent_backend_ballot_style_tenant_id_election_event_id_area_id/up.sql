alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_tenant_id_election_event_id_area_id_fkey"
  foreign key ("tenant_id", "election_event_id", "area_id")
  references "sequent_backend"."area"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;
