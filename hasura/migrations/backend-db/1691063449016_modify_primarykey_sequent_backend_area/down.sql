alter table "sequent_backend"."area" drop constraint "area_pkey";
alter table "sequent_backend"."area"
    add constraint "area_pkey"
    primary key ("id");
