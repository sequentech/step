alter table "sequent_backend"."communication_template"
  add constraint "communication_template_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
