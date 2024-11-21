alter table "sequent_backend"."results_area_contest" drop constraint "results_area_contest_tenant_id_election_event_id_election_id",
  add constraint "results_area_contest_tenant_id_id_election_event_id_fkey"
  foreign key ("id", "election_event_id", "tenant_id")
  references "sequent_backend"."election"
  ("id", "election_event_id", "tenant_id") on update restrict on delete restrict;
