// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel} from "@/types/messaging"
import {electoralLogChannel} from "./electoralLogChannel"

const entry = (statement: unknown) => JSON.stringify({statement})

describe("electoralLogChannel", () => {
    it("reads the channel of a SendCommunications body", () => {
        expect(
            electoralLogChannel(
                entry({
                    head: {kind: "SendCommunications"},
                    body: {SendCommunications: {channel: "VIBER", purpose: "NOTICE"}},
                })
            )
        ).toBe(EMessageChannel.VIBER)
    })

    it("reads a body serialized as text", () => {
        expect(
            electoralLogChannel(
                entry({
                    head: {kind: "SendCommunications"},
                    body: {SendCommunications: JSON.stringify({channel: "WHATSAPP"})},
                })
            )
        ).toBe(EMessageChannel.WHATSAPP)
    })

    it("reads a channel in the statement head", () => {
        expect(electoralLogChannel(entry({head: {channel: "SMS"}, body: {}}))).toBe(
            EMessageChannel.SMS
        )
    })

    it("has no channel for other statements, plain text bodies or malformed entries", () => {
        expect(electoralLogChannel(entry({head: {kind: "CastVote"}, body: {CastVote: []}}))).toBe(
            undefined
        )
        expect(
            electoralLogChannel(entry({head: {}, body: {SendCommunications: "template body"}}))
        ).toBe(undefined)
        expect(electoralLogChannel(entry({head: {channel: "FAX"}}))).toBe(undefined)
        expect(electoralLogChannel("{not json")).toBe(undefined)
        expect(electoralLogChannel(undefined)).toBe(undefined)
    })
})
