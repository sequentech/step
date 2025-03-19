CREATE OR REPLACE FUNCTION sequent_backend.search_applications_fn(
  p_election_event_id UUID,
  permission_label TEXT DEFAULT NULL,
  regular_filters JSONB DEFAULT '{}',
  jsonb_filters JSONB DEFAULT '{}',
  sort_field TEXT DEFAULT 'created_at',
  sort_order TEXT DEFAULT 'ASC',
  per_page INT DEFAULT 10,
  offset_value INT DEFAULT 0
)
RETURNS TABLE (  -- Explicit table structure
  id UUID,
  election_event_id UUID,
  applicant_id UUID,
  verification_type TEXT,
  status TEXT,
  applicant_data JSONB,
  created_at TIMESTAMP,
  updated_at TIMESTAMP
)
LANGUAGE plpgsql
AS $$
BEGIN
  RETURN QUERY
  SELECT 
    app.id,
    app.election_event_id,
    app.applicant_id,
    app.verification_type,
    app.status,
    app.applicant_data,
    app.created_at,
    app.updated_at
  FROM sequent_backend.applications AS app
  WHERE 
    app.election_event_id = p_election_event_id
    AND (permission_label IS NULL OR permission_label = app.permission_label)

    
    -- Regular column filters
    AND (regular_filters->>'id' IS NULL OR app.id::text = regular_filters->>'id')
    AND (regular_filters->>'applicant_id' IS NULL OR app.applicant_id::text = regular_filters->>'applicant_id')
    AND (regular_filters->>'verification_type' IS NULL OR app.verification_type ILIKE '%' || (regular_filters->>'verification_type') || '%')
    AND (regular_filters->>'status' IS NULL OR app.status ILIKE '%' || (regular_filters->>'status') || '%')

    -- JSONB filters for applicant_data
    AND (
      jsonb_filters = '{}'::jsonb
      OR NOT EXISTS (
        SELECT 1 
        FROM jsonb_each_text(jsonb_filters) AS jf(key, value)
        WHERE NOT (
          (jf.key = 'dateOfBirth' AND app.applicant_data->>jf.key = jf.value)
          OR (jf.key != 'dateOfBirth' AND app.applicant_data->>jf.key ILIKE '%' || jf.value || '%')
        )
      )
    )
  ORDER BY 
    CASE 
      WHEN sort_order = 'ASC' THEN
        CASE 
          WHEN sort_field = 'id' THEN app.id::text
          WHEN sort_field = 'created_at' THEN TO_CHAR(app.created_at, 'YYYY-MM-DD HH:MI:SS.US')
          WHEN sort_field = 'updated_at' THEN TO_CHAR(app.updated_at, 'YYYY-MM-DD HH:MI:SS.US')
          WHEN sort_field = 'applicant_id' THEN app.applicant_id::text
          WHEN sort_field = 'verification_type' THEN app.verification_type::text
          WHEN sort_field = 'status' THEN app.status::text
          ELSE TO_CHAR(app.created_at, 'YYYY-MM-DD HH:MI:SS.US')
        END
      END ASC,
    CASE 
      WHEN sort_order = 'DESC' THEN
        CASE 
          WHEN sort_field = 'id' THEN app.id::text
          WHEN sort_field = 'created_at' THEN TO_CHAR(app.created_at, 'YYYY-MM-DD HH:MI:SS.US')
          WHEN sort_field = 'updated_at' THEN TO_CHAR(app.updated_at, 'YYYY-MM-DD HH:MI:SS.US')
          WHEN sort_field = 'applicant_id' THEN app.applicant_id::text
          WHEN sort_field = 'verification_type' THEN app.verification_type::text
          WHEN sort_field = 'status' THEN app.status::text
          ELSE TO_CHAR(app.created_at, 'YYYY-MM-DD HH:MI:SS.US')
        END
      END DESC
  LIMIT per_page
  OFFSET offset_value;
END;
$$;
