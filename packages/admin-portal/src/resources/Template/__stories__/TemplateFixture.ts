// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic communication and report templates of the tenant.
import type {Sequent_Backend_Template} from "@/gql/graphql"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, storyId, type StoryRecord} from "@/__stories__/fixtures"
import {ETemplateType, ITemplateMethod} from "@/types/templates"

export const TEMPLATE_RESOURCE = "sequent_backend_template"
export const CREDENTIALS_TEMPLATE_ID = storyId(7, 1)
export const RECEIPT_TEMPLATE_ID = storyId(7, 2)

const template = (
    id: string,
    alias: string,
    name: string,
    type: ETemplateType,
    content: Record<string, unknown>
): StoryRecord<Sequent_Backend_Template> => ({
    id,
    alias,
    tenant_id: TENANT_ID,
    type,
    communication_method: ITemplateMethod.EMAIL,
    template: {alias, name, ...content},
    created_by: "admin",
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
    annotations: {},
    labels: {},
})

export const templateRecords = (): StoryRecord<Sequent_Backend_Template>[] => [
    template(
        CREDENTIALS_TEMPLATE_ID,
        "voter-credentials",
        "Voter credentials",
        ETemplateType.CREDENTIALS,
        {
            selected_methods: {EMAIL: true, SMS: true, DOCUMENT: false},
            email: {
                subject: "Your voting credentials",
                plaintext_body: "Vote at {{vote_url}}",
                html_body: "<p>Vote at {{vote_url}}</p>",
            },
            sms: {message: "Vote at {{vote_url}}"},
        }
    ),
    template(
        RECEIPT_TEMPLATE_ID,
        "ballot-receipt",
        "Ballot receipt",
        ETemplateType.BALLOT_RECEIPT,
        {
            selected_methods: {EMAIL: false, SMS: false, DOCUMENT: true},
            document: "<h1>Receipt {{ballot_id}}</h1>",
            pdf_options: {format: "A4"},
            report_options: {max_items_per_report: 100},
        }
    ),
]
