alter table "sequent_backend"."election"
  add constraint "election_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
