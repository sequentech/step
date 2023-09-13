alter table "sequent_backend"."trustee"
  add constraint "trustee_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
