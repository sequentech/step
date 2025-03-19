-- Check if the type exists and recreate if needed
DROP TYPE IF EXISTS sequent_backend.application_count_type CASCADE;
CREATE TYPE sequent_backend.application_count_type AS (
  total INTEGER
);
