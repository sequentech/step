// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EChannelSelection, EMessageChannel} from "@/types/messaging"
import {ITemplateMethod} from "@/types/templates"
import {
    buildSendTemplatePayload,
    missingChannelContent,
    templatesForChannelSelection,
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
    it("sends each voter's channel without a single method", () => {
        const payload = buildSendTemplatePayload({
            ...base,
            channelSelection: EChannelSelection.VOTER_PREFERENCE,
            communicationMethod: ITemplateMethod.SMS,
        })
        expect(payload.channel_selection).toBe(EChannelSelection.VOTER_PREFERENCE)
        expect(payload.communication_method).toBeUndefined()
        expect(payload.email).toEqual(content.email)
        expect(payload.sms).toEqual(content.sms)
        expect(payload.whatsapp).toEqual(content.whatsapp)
        expect(payload.viber).toBeUndefined()
        expect(payload.messenger).toEqual(content.messenger)
        expect(payload.audience_voter_ids).toEqual(["voter-1"])
        expect(payload.secret_attribute_names).toEqual(["passport"])
    })

    it("sends one channel with only that channel's content", () => {
        const payload = buildSendTemplatePayload({
            ...base,
            channelSelection: EChannelSelection.SINGLE_CHANNEL,
            communicationMethod: ITemplateMethod.WHATSAPP,
        })
        expect(payload.channel_selection).toBe(EChannelSelection.SINGLE_CHANNEL)
        expect(payload.communication_method).toBe(ITemplateMethod.WHATSAPP)
        expect(payload.whatsapp).toEqual(content.whatsapp)
        expect(payload.email).toBeUndefined()
        expect(payload.sms).toBeUndefined()
        expect(payload.messenger).toBeUndefined()
    })
})

describe("templatesForChannelSelection", () => {
    const rows = [
        {
            communication_method: "EMAIL",
            template: {alias: "both", selected_methods: {EMAIL: true, SMS: true}},
        },
        {communication_method: "SMS", template: {alias: "legacy-sms"}},
        {communication_method: "DOCUMENT", template: {alias: "receipt", selected_methods: {}}},
    ]

    it("offers SMS templates for an SMS send, including multi-method ones", () => {
        expect(
            templatesForChannelSelection(
                rows,
                EChannelSelection.SINGLE_CHANNEL,
                ITemplateMethod.SMS
            ).map((row) => row.template.alias)
        ).toEqual(["both", "legacy-sms"])
    })

    it("offers every messaging template for each voter's channel", () => {
        expect(
            templatesForChannelSelection(
                rows,
                EChannelSelection.VOTER_PREFERENCE,
                ITemplateMethod.EMAIL
            ).map((row) => row.template.alias)
        ).toEqual(["both", "legacy-sms"])
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
