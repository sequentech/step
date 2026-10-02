-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DO $$ DECLARE table_name text; BEGIN
  FOREACH table_name IN ARRAY ARRAY['template','report','election_event','election','contest','candidate','ballot_publication','tally_results_publication'] LOOP
    EXECUTE format('DROP TRIGGER IF EXISTS invalidate_report_prerender ON sequent_backend.%I',table_name);
  END LOOP;
END $$;
DROP FUNCTION sequent_backend.invalidate_report_prerenders();
DROP FUNCTION sequent_backend.queue_report_prerenders(uuid,uuid,text);
DROP TABLE sequent_backend.report_prerender;
