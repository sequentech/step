alter table "sequent_backend"."contest" drop constraint "contest_pkey";
alter table "sequent_backend"."contest"
    add constraint "contest_pkey"
    primary key ("id");
