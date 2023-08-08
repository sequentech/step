alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;
