# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
def board_only: test("^((sign|voter|contest|candidate|area|application|document"
  + "|report|miru|tally|publish|logs|electoral-log|signing-requests|monitoring"
  + "|scheduled-event)-.+|election-event-(archive|create|delete|keys-tab"
  + "|logs-tab|publish-(preview|tab|view)|reports-tab|scheduled-(tab|event-columns)"
  + "|tally-tab|tasks-tab|voter-list-reconciliation|voters-tab)|election-(create"
  + "|delete|ip-address-view|publish-tab|state-write|voters-tab)"
  + "|(admin|create|export|transmition|trustee)-ceremony|admin-ip-address-view"
  + "|datafix-account|ee-voters-logs|keys-read|task-export|tasks-read"
  + "|transmission-report-generate|user-import)$");
