-- Remove conflicting objects
DROP FUNCTION IF EXISTS sequent_backend.count_applications_func CASCADE;
DROP FUNCTION IF EXISTS sequent_backend.count_applications CASCADE;
DROP TYPE IF EXISTS sequent_backend.application_count CASCADE;
