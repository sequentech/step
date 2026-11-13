// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface ExportSelection {
    bulletinBoard: boolean
    activityLogs: boolean
    publications: boolean
    reports: boolean
    applications: boolean
}

/** Mirrors windmill's `ExportOptions::includes_protocol_manager_keys`. */
export const includesProtocolManagerKeys = ({
    bulletinBoard,
    activityLogs,
    publications,
}: ExportSelection): boolean => bulletinBoard || activityLogs || publications

/** Mirrors windmill's `ExportOptions::requires_encryption`. */
export const requiresEncryption = (selection: ExportSelection): boolean =>
    includesProtocolManagerKeys(selection) || selection.reports || selection.applications
