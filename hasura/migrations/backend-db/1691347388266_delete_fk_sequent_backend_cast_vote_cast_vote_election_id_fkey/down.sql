alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;
