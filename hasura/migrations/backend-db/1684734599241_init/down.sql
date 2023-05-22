
DROP INDEX IF EXISTS "sequent_backend"."tenant_labels";

DROP INDEX IF EXISTS "sequent_backend"."event_labels";

DROP TABLE "sequent_backend"."event";

DROP TABLE "sequent_backend"."tenant";

drop schema "sequent_backend" cascade;
