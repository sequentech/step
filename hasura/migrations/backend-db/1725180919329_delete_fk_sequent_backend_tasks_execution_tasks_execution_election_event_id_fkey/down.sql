alter table "sequent_backend"."tasks_execution"
  add constraint "tasks_execution_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;
