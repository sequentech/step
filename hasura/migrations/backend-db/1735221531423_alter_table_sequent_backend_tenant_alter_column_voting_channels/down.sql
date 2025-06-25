alter table "sequent_backend"."tenant" alter column "voting_channels" set default '{"kiosk": true, "online": true}'::jsonb;
