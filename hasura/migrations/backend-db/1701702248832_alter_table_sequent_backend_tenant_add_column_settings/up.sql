alter table "sequent_backend"."tenant" add column "settings" jsonb
 null;

UPDATE "sequent_backend"."tenant"
SET settings = '{"i18n":{"en":{}},"help_links":[{"url":"${PUBLIC_BUCKET_URL}public-assets/system-manual.pdf","i18n":{"en":{"title":"System Manual"},"es":{"title":"Manual del Sistema"},"fr":{"title":"System Manual"},"tl":{"title":"System Manual"},"cat":{"title":"System Manual"}},"title":"System Manual"}],"language_conf":{"default_language_code":"en","enabled_language_codes":["en"]},"enroll_countries":[],"voting_countries":[]}',
annotations = '{}'
WHERE id = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

