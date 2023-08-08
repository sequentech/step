alter table "sequent_backend"."contest" drop constraint "contest_tenant_id_fkey",
  add constraint "contest_id_fkey"
  foreign key ("id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
