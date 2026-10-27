-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.signing_rule DROP CONSTRAINT signing_rule_action_known;
ALTER TABLE sequent_backend.signing_rule ADD CONSTRAINT signing_rule_action_known CHECK (action IN (
    'initialize-voting', 'open-voting', 'close-voting', 'generate-election-returns',
    'generate-reports', 'transmit-results', 'approve-voter', 'approve-configuration',
    'key-ceremony', 'tally-key', 'import-election-data', 'start-tally', 'export-execution-data',
    'delete-execution-data', 'read-voters-register'
));
ALTER TABLE sequent_backend.signing_request DROP CONSTRAINT signing_request_action_known;
ALTER TABLE sequent_backend.signing_request ADD CONSTRAINT signing_request_action_known CHECK (action IN (
    'initialize-voting', 'open-voting', 'close-voting', 'generate-election-returns',
    'generate-reports', 'transmit-results', 'approve-voter', 'approve-configuration',
    'key-ceremony', 'tally-key', 'import-election-data', 'start-tally', 'export-execution-data',
    'delete-execution-data', 'read-voters-register'
));
-- Existing disabled rules remain readable for administrator correction. They
-- fail closed in the signing guard; every new/updated required rule needs quorum.
ALTER TABLE sequent_backend.signing_rule ADD CONSTRAINT signing_rule_mandatory_actions CHECK (
    action NOT IN ('open-voting', 'close-voting', 'approve-configuration', 'import-election-data', 'start-tally',
        'export-execution-data', 'delete-execution-data', 'read-voters-register')
    OR requirement = 'required'
) NOT VALID;
