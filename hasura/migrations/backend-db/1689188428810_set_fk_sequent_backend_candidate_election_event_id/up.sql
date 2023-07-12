alter table "sequent_backend"."candidate"
  add constraint "candidate_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;
