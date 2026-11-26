-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Each enrollment operation is a separate rule. No event is changed by this migration.
ALTER TABLE sequent_backend.signing_rule
    DROP CONSTRAINT signing_rule_action_known,
    ADD CONSTRAINT signing_rule_action_known CHECK (action IN (
        'initialize-voting', 'open-voting', 'close-voting',
        'generate-election-returns', 'generate-reports', 'transmit-results',
        'approve-voter', 'authorize-manual-verification', 'approve-configuration',
        'key-ceremony', 'tally-key'
    ));
ALTER TABLE sequent_backend.signing_request
    DROP CONSTRAINT signing_request_action_known,
    ADD CONSTRAINT signing_request_action_known CHECK (action IN (
        'initialize-voting', 'open-voting', 'close-voting',
        'generate-election-returns', 'generate-reports', 'transmit-results',
        'approve-voter', 'authorize-manual-verification', 'approve-configuration',
        'key-ceremony', 'tally-key'
    ));
