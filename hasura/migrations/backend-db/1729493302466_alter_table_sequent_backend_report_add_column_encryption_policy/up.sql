alter table "sequent_backend"."report" add column "encryption_policy" character varying
 not null default 'unencrypted';
