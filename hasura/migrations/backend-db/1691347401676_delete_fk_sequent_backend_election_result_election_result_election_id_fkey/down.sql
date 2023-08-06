alter table "sequent_backend"."election_result"
  add constraint "election_result_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;
