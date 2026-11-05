// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Reports of the election event: an unencrypted activity log for the whole
// event, a password-protected participation report for the council election
// that uses a custom template, and a council initialization report.
import type {Sequent_Backend_Report, Sequent_Backend_Template} from "@/gql/graphql"
import {FIXED_TIME, STORY_IDS, electionRecord, storyId} from "@/__stories__/fixtures"
import {IPermissions} from "@/types/keycloak"
import {EReportType} from "@/types/reports"
import {ETemplateType} from "@/types/templates"
import {EReportEncryption} from "../EditReportForm"

export const REPORT_IDS = {
    activityLogs: storyId(1, 1),
    participation: storyId(1, 2),
    initialization: storyId(1, 3),
}

export const PARTICIPATION_TEMPLATE = "participation-summary"

const scope = {tenant_id: STORY_IDS.tenant, election_event_id: STORY_IDS.event}

export function reportRecord(overrides: Partial<Sequent_Backend_Report> = {}) {
    return {
        ...scope,
        id: REPORT_IDS.activityLogs,
        report_type: EReportType.ACTIVITY_LOGS,
        election_id: null,
        template_alias: null,
        encryption_policy: EReportEncryption.UNENCRYPTED,
        cron_config: null,
        permission_label: null,
        created_at: FIXED_TIME,
        ...overrides,
    } as Sequent_Backend_Report
}

export const REPORTS = [
    reportRecord(),
    reportRecord({
        id: REPORT_IDS.participation,
        report_type: EReportType.PARTICIPATION_REPORT,
        election_id: STORY_IDS.election,
        template_alias: PARTICIPATION_TEMPLATE,
        encryption_policy: EReportEncryption.CONFIGURED_PASSWORD,
        permission_label: ["north"],
        cron_config: {
            is_active: true,
            cron_expression: "0 8 * * 1",
            email_recipients: ["observer@example.org"],
            executer_username: "admin",
        },
    }),
    reportRecord({
        id: REPORT_IDS.initialization,
        report_type: EReportType.INITIALIZATION_REPORT,
        election_id: STORY_IDS.election,
    }),
]

export const TEMPLATES = [
    {
        id: storyId(2, 1),
        tenant_id: STORY_IDS.tenant,
        alias: PARTICIPATION_TEMPLATE,
        type: ETemplateType.PARTICIPATION_REPORT,
        communication_method: "EMAIL",
        template: {name: "Participation summary"},
        created_by: "admin",
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        labels: {},
        annotations: {},
    },
] as Sequent_Backend_Template[]

export const ELECTIONS = [electionRecord()]

export const REPORT_ROLES = [
    IPermissions.REPORT_READ,
    IPermissions.REPORT_WRITE,
    IPermissions.REPORT_CREATE,
    IPermissions.REPORT_DELETE,
    IPermissions.REPORT_GENERATE,
    IPermissions.REPORT_PREVIEW,
    IPermissions.DOCUMENT_PASSWORD_READ,
    IPermissions.DOCUMENT_DOWNLOAD,
]

/** Axe defects of the react-js-cron schedule editor a repeatable report shows. */
export const scheduleDefects = {
    reason:
        "The schedule editor's selects have no label and its clear button has white text " +
        "on #ff4d4f, below 4.5 contrast.",
    a11y: ["color-contrast", "label"],
}
