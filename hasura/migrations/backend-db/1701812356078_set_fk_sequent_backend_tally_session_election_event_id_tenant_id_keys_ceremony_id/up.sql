alter table "sequent_backend"."tally_session"
  add constraint "tally_session_election_event_id_tenant_id_keys_ceremony_id_f"
  foreign key ("election_event_id", "tenant_id", "keys_ceremony_id")
  references "sequent_backend"."keys_ceremony"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
