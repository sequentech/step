-- 1. Drop existing function if it exists
DROP FUNCTION IF EXISTS sequent_backend.count_applications_func(uuid, text, jsonb, jsonb);

-- 2. Create TRACKABLE function with proper aliasing
CREATE OR REPLACE FUNCTION sequent_backend.count_applications_func(
  election_event_id UUID,
  permission_label TEXT DEFAULT NULL,
  regular_filters JSONB DEFAULT '{}'::jsonb,
  jsonb_filters JSONB DEFAULT '{}'::jsonb
)
RETURNS TABLE(total INTEGER)  -- Explicit table return type
LANGUAGE sql
STABLE
AS $$
  SELECT COUNT(*) AS total
  FROM sequent_backend.applications AS app  -- Added table alias
  WHERE 
    app.election_event_id = $1  -- Qualified with alias
    
    -- Fixed parameter vs column ambiguity
    AND (permission_label IS NULL OR permission_label = app.permission_label)
    
    -- Regular filters (qualified with alias)
    AND (regular_filters->>'id' IS NULL OR app.id::TEXT = (regular_filters->>'id')::TEXT)
    AND (regular_filters->>'applicant_id' IS NULL OR app.applicant_id::TEXT = (regular_filters->>'applicant_id')::TEXT)
    AND (regular_filters->>'verification_type' IS NULL OR app.verification_type ILIKE ('%' || (regular_filters->>'verification_type') || '%'))
    AND (regular_filters->>'status' IS NULL OR app.status ILIKE ('%' || (regular_filters->>'status') || '%'))

    -- JSONB filters (qualified with alias)
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
