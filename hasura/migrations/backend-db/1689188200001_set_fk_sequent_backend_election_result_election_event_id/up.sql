alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;
