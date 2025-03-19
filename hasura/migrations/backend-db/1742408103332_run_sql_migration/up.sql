CREATE OR REPLACE FUNCTION sequent_backend.search_applications_func(p_election_event_id uuid, permission_label text DEFAULT NULL::text, regular_filters jsonb DEFAULT '{}'::jsonb, jsonb_filters jsonb DEFAULT '{}'::jsonb, sort_field text DEFAULT 'created_at'::text, sort_order text DEFAULT 'ASC'::text, per_page integer DEFAULT 10, offset_value integer DEFAULT 0)
 RETURNS SETOF sequent_backend.applications
 LANGUAGE sql
 STABLE
AS $$
  SELECT *
  FROM sequent_backend.applications AS app
  WHERE 
    app.election_event_id = p_election_event_id
    AND (permission_label IS NULL OR permission_label = app.permission_label)

    -- Regular filters (SAFE TYPE HANDLING)
    AND (regular_filters->>'id' IS NULL OR app.id::TEXT = (regular_filters->>'id')::TEXT)
    AND (regular_filters->>'applicant_id' IS NULL OR app.applicant_id::TEXT = (regular_filters->>'applicant_id')::TEXT)
    AND (regular_filters->>'verification_type' IS NULL OR app.verification_type ILIKE ('%' || (regular_filters->>'verification_type') || '%'))
    AND (regular_filters->>'status' IS NULL OR app.status ILIKE ('%' || (regular_filters->>'status') || '%'))

    -- JSONB filters (CORRECTED LOGIC)
    AND (
      jsonb_filters = '{}'::jsonb OR EXISTS (
        SELECT 1
        FROM jsonb_each_text(jsonb_filters) AS jf(key, value)
        WHERE 
          (jf.key = 'dateOfBirth' AND app.applicant_data->>jf.key = jf.value)
          OR 
          (jf.key <> 'dateOfBirth' AND app.applicant_data->>jf.key ILIKE ('%' || jf.value || '%'))
      )
    )

  -- FIXED SORTING (USING CONDITIONAL MULTIPLIERS)
  ORDER BY 
    CASE 
      WHEN sort_order = 'ASC' THEN
        CASE sort_field
          WHEN 'id' THEN app.id::TEXT
          WHEN 'created_at' THEN app.created_at::TEXT
          WHEN 'updated_at' THEN app.updated_at::TEXT
          WHEN 'applicant_id' THEN app.applicant_id::TEXT
          WHEN 'verification_type' THEN app.verification_type
          WHEN 'status' THEN app.status
        END
    END ASC,
    CASE 
      WHEN sort_order = 'DESC' THEN
        CASE sort_field
          WHEN 'id' THEN app.id::TEXT
          WHEN 'created_at' THEN app.created_at::TEXT
          WHEN 'updated_at' THEN app.updated_at::TEXT
          WHEN 'applicant_id' THEN app.applicant_id::TEXT
          WHEN 'verification_type' THEN app.verification_type
          WHEN 'status' THEN app.status
        END
    END DESC

  LIMIT per_page
  OFFSET offset_value;
$$;
