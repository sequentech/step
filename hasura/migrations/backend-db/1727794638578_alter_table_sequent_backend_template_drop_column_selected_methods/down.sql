alter table "sequent_backend"."template" alter column "selected_methods" drop not null;
alter table "sequent_backend"."template" add column "selected_methods" jsonb;
