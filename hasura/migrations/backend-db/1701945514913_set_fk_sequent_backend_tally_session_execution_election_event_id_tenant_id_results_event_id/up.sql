alter table "sequent_backend"."tally_session_execution"
  add constraint "tally_session_execution_election_event_id_tenant_id_results_"
  foreign key ("election_event_id", "tenant_id", "results_event_id")
  references "sequent_backend"."results_event"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
