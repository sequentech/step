alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_session_contest_tenant_id_election_event_id_election_i"
  foreign key ("tenant_id", "election_event_id", "election_id")
  references "sequent_backend"."election"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;
