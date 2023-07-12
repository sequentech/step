alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;
