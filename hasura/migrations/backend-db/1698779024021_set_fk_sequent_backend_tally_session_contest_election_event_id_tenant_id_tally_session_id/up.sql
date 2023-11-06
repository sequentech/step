alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_session_contest_election_event_id_tenant_id_tally_sess"
  foreign key ("election_event_id", "tenant_id", "tally_session_id")
  references "sequent_backend"."tally_session"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
