alter table "sequent_backend"."report" alter column "template_id" drop not null;
alter table "sequent_backend"."report" add column "template_id" text;
