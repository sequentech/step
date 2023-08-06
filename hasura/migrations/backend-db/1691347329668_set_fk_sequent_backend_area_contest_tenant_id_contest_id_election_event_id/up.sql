alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_contest_id_election_event_id_fkey"
  foreign key ("tenant_id", "contest_id", "election_event_id")
  references "sequent_backend"."contest"
  ("tenant_id", "id", "election_id") on update restrict on delete restrict;
