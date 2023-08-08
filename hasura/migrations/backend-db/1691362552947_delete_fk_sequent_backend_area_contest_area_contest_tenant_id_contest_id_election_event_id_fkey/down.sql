alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_contest_id_election_event_id_fkey"
  foreign key ("election_event_id", "contest_id", "tenant_id")
  references "sequent_backend"."contest"
  ("election_id", "id", "tenant_id") on update restrict on delete restrict;
