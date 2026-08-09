DROP TRIGGER IF EXISTS "guard_results_website_policy_update"
ON "sequent_backend"."election_event";

DROP FUNCTION IF EXISTS "sequent_backend"."guard_results_website_policy_update"();
