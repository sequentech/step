alter table "sequent_backend"."contest"
  add constraint "contest_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
