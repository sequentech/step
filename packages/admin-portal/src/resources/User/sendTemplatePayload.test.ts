// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel} from "@/types/messaging"
import {
    buildSendTemplatePayload,
    missingChannelContent,
    sendableTemplates,
} from "./sendTemplatePayload"

const content = {
    email: {subject: "Hello", plaintext_body: "Hi", html_body: "<p>Hi</p>"},
    sms: {message: "Hi {{vote_url}}"},
    whatsapp: {message: "Approved wording", parameters: ["user.first_name"]},
    messenger: {message: "Hi in Messenger", parameters: []},
}

const base = {
    audienceSelection: "SELECTED",
    voterIds: ["voter-1"],
    scheduleNow: true,
    scheduleDate: undefined,
    content,
    secretAttributeNames: ["passport"],
}

describe("buildSendTemplatePayload", () => {
    it("sends the content of every channel, with no channel choice", () => {
        const payload = buildSendTemplatePayload(base)
        expect("channel_selection" in payload).toBe(false)
        expect("communication_method" in payload).toBe(false)
        expect(payload.email).toEqual(content.email)
        expect(payload.sms).toEqual(content.sms)
        expect(payload.whatsapp).toEqual(content.whatsapp)
        expect(payload.viber).toBeUndefined()
        expect(payload.messenger).toEqual(content.messenger)
        expect(payload.audience_voter_ids).toEqual(["voter-1"])
        expect(payload.secret_attribute_names).toEqual(["passport"])
    })

    it("names the provider's template and language when they are set", () => {
        const payload = buildSendTemplatePayload({
            ...base,
            content: {
                ...content,
                whatsapp: {
                    ...content.whatsapp,
                    provider_template: " vote_reminder ",
                    provider_language: " en_US ",
                },
                viber: {message: "Approved", parameters: [], provider_template: "88213"},
                messenger: {...content.messenger, provider_template: " ", provider_language: null},
            },
        })
        expect(payload.whatsapp).toEqual({
            ...content.whatsapp,
            provider_template: "vote_reminder",
            provider_language: "en_US",
        })
        expect(payload.viber).toEqual({
            message: "Approved",
            parameters: [],
            provider_template: "88213",
        })
        expect(payload.messenger).toEqual(content.messenger)
        expect("alias" in payload).toBe(false)
    })

    it("carries the alias of the chosen template, the key its template is bound under", () => {
        const payload = buildSendTemplatePayload({
            ...base,
            alias: "reminder",
        })
        expect(payload.alias).toBe("reminder")
    })
})

describe("sendableTemplates", () => {
    const rows = [
        {
            communication_method: "EMAIL",
            template: {alias: "both", selected_methods: {EMAIL: true, SMS: true}},
        },
        {communication_method: "SMS", template: {alias: "legacy-sms"}},
        {communication_method: "DOCUMENT", template: {alias: "receipt", selected_methods: {}}},
    ]

    it("offers every template that sends a message, not documents", () => {
        expect(sendableTemplates(rows).map((row) => row.template.alias)).toEqual([
            "both",
            "legacy-sms",
        ])
    })
})

describe("missingChannelContent", () => {
    it("lists the enabled channels the content does not cover", () => {
        expect(
            missingChannelContent(content, [
                EMessageChannel.EMAIL,
                EMessageChannel.VIBER,
                EMessageChannel.MESSENGER,
            ])
        ).toEqual([EMessageChannel.VIBER])
    })

    it("treats empty messages as missing", () => {
        expect(missingChannelContent({sms: {message: "  "}}, [EMessageChannel.SMS])).toEqual([
            EMessageChannel.SMS,
        ])
    })
})
