alter table "sequent_backend"."report" alter column "template_alias" drop not null;
alter table "sequent_backend"."report" add column "template_alias" varchar;
