alter table "sequent_backend"."contest"
  add constraint "contest_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;
