alter table "sequent_backend"."scheduled_event" alter column "cron_config" drop not null;
alter table "sequent_backend"."scheduled_event" add column "cron_config" varchar;
