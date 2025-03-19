DROP FUNCTION IF EXISTS sequent_backend.count_applications_func;

CREATE OR REPLACE FUNCTION sequent_backend.count_applications_func(
  election_event_id UUID
)
RETURNS TABLE(total BIGINT) 
LANGUAGE sql
STABLE
AS $$
  SELECT COUNT(*)::BIGINT
  FROM sequent_backend.applications
  WHERE election_event_id = $1;
$$;
