alter table "sequent_backend"."contest" alter column "order_answers" drop not null;
alter table "sequent_backend"."contest" add column "order_answers" text;
