alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_id", "tenant_id", "election_event_id")
  references "sequent_backend"."election"
  ("id", "tenant_id", "election_event_id") on update restrict on delete restrict;
