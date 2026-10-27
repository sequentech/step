-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Never silently remove accepted authorisations during a rollback.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM sequent_backend.signing_request WHERE action IN
        ('import-election-data', 'start-tally','export-execution-data','delete-execution-data','read-voters-register')) THEN
        RAISE EXCEPTION 'Cannot roll back while new board-action authorisations exist';
    END IF;
END $$;
ALTER TABLE sequent_backend.signing_rule DROP CONSTRAINT signing_rule_mandatory_actions;
ALTER TABLE sequent_backend.signing_rule DROP CONSTRAINT signing_rule_action_known;
ALTER TABLE sequent_backend.signing_rule ADD CONSTRAINT signing_rule_action_known CHECK (action IN (
    'initialize-voting','open-voting','close-voting','generate-election-returns','generate-reports',
    'transmit-results','approve-voter','approve-configuration','key-ceremony','tally-key'
));
ALTER TABLE sequent_backend.signing_request DROP CONSTRAINT signing_request_action_known;
ALTER TABLE sequent_backend.signing_request ADD CONSTRAINT signing_request_action_known CHECK (action IN (
    'initialize-voting','open-voting','close-voting','generate-election-returns','generate-reports',
    'transmit-results','approve-voter','approve-configuration','key-ceremony','tally-key'
));
