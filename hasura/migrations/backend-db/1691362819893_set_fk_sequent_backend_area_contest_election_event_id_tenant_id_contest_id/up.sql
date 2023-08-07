alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_contest_id_fkey"
  foreign key ("election_event_id", "tenant_id", "contest_id")
  references "sequent_backend"."contest"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
