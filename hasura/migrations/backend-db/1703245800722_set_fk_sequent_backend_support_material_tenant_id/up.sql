alter table "sequent_backend"."support_material"
  add constraint "support_material_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
