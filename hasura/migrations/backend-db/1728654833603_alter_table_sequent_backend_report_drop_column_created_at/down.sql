alter table "sequent_backend"."report" alter column "created_at" set default now();
alter table "sequent_backend"."report" alter column "created_at" drop not null;
alter table "sequent_backend"."report" add column "created_at" timetz;
