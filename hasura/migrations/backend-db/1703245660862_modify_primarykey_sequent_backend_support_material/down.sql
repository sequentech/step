alter table "sequent_backend"."support_material" drop constraint "support_material_pkey";
alter table "sequent_backend"."support_material"
    add constraint "support_material_pkey"
    primary key ("id");
