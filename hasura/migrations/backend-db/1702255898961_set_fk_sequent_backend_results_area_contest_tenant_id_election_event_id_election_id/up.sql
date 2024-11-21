alter table "sequent_backend"."results_area_contest" drop constraint "results_area_contest_tenant_id_id_election_event_id_fkey",
  add constraint "results_area_contest_tenant_id_election_event_id_election_id"
  foreign key ("tenant_id", "election_event_id", "election_id")
  references "sequent_backend"."election"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;
