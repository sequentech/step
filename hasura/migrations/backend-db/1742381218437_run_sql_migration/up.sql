DROP FUNCTION IF EXISTS sequent_backend.count_applications_func;

-- First create a dedicated type for the return value
DROP TYPE IF EXISTS sequent_backend.count_type;
CREATE TYPE sequent_backend.count_type AS (
  total bigint
);

-- Create the function with explicit casting to the type
CREATE OR REPLACE FUNCTION sequent_backend.count_applications_func(
  election_event_id UUID,
  permission_label TEXT DEFAULT NULL,
  regular_filters JSONB DEFAULT '{}'::jsonb,
  jsonb_filters JSONB DEFAULT '{}'::jsonb
)
RETURNS SETOF sequent_backend.count_type
LANGUAGE sql
VOLATILE -- Try VOLATILE instead of STABLE
AS $$
  SELECT ROW(COUNT(*)::BIGINT)::sequent_backend.count_type
  FROM sequent_backend.applications AS app
  WHERE 
    app.election_event_id = election_event_id
    AND (permission_label IS NULL OR app.permission_label = permission_label)
    
    -- Regular filters
    AND (regular_filters->>'id' IS NULL OR app.id::TEXT = (regular_filters->>'id')::TEXT)
    AND (regular_filters->>'applicant_id' IS NULL OR app.applicant_id::TEXT = (regular_filters->>'applicant_id')::TEXT)
    AND (regular_filters->>'verification_type' IS NULL OR app.verification_type ILIKE ('%' || (regular_filters->>'verification_type') || '%'))
    AND (regular_filters->>'status' IS NULL OR app.status ILIKE ('%' || (regular_filters->>'status') || '%'))
    -- JSONB filters
    AND (
      jsonb_filters = '{}'::jsonb OR
      NOT EXISTS (
        SELECT 1 
        FROM jsonb_each_text(jsonb_filters) AS jf(key, value)
        WHERE NOT (
          (jf.key = 'dateOfBirth' AND app.applicant_data->>jf.key = jf.value)
          OR 
          (jf.key <> 'dateOfBirth' AND app.applicant_data->>jf.key ILIKE ('%' || jf.value || '%'))
        )
      )
    );
$$;
