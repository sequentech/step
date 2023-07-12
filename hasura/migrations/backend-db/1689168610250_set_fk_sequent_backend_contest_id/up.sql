alter table "sequent_backend"."contest"
  add constraint "contest_id_fkey"
  foreign key ("id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
