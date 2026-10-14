// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel} from "@/types/messaging"
import {electoralLogChannel, electoralLogDelivery} from "./electoralLogChannel"

const entry = (statement: unknown) => JSON.stringify({statement})

/**
 * A SendCommunications entry exactly as windmill writes it: `ledger_audit_message` (and
 * `delivery_audit_message`) put the channel in lowercase inside the JSON string the
 * Borsh `SendCommunications(Option<String>)` body carries (AP-B1).
 */
const windmillEntry = (details: Record<string, unknown>) =>
    JSON.stringify({
        statement: {
            head: {
                event: "8a5c1d4e-6f0b-4c5b-9a3e-1d2f3a4b5c6d",
                kind: "SendCommunications",
                timestamp: 1791200000,
                event_type: "INFO",
                log_type: "INFO",
                description: "Communication sent to user.",
            },
            body: {SendCommunications: JSON.stringify(details)},
        },
        user_id: "4b9c1a2d-0000-4000-8000-000000000001",
    })

describe("electoralLogChannel", () => {
    it("reads windmill's lowercase channel from a real SendCommunications record", () => {
        expect(
            electoralLogChannel(
                windmillEntry({
                    channel: "whatsapp",
                    receiver: "+34•••214",
                    message_id: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00",
                    state: "ACCEPTED",
                })
            )
        ).toBe(EMessageChannel.WHATSAPP)
        expect(electoralLogChannel(windmillEntry({channel: "messenger"}))).toBe(
            EMessageChannel.MESSENGER
        )
    })

    it("reads the message, outcome, attempt and reason windmill records", () => {
        expect(
            electoralLogDelivery(
                windmillEntry({
                    channel: "viber",
                    receiver: "+63•••567",
                    message_id: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00",
                    attempt: 2,
                    outcome: "FAILED",
                    reason: "No contact channel",
                })
            )
        ).toEqual({
            channel: EMessageChannel.VIBER,
            messageId: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00",
            outcome: "FAILED",
            attempt: 2,
            reason: "No contact channel",
        })
        // Entries written before the second pass call the outcome `state`.
        expect(
            electoralLogDelivery(windmillEntry({channel: "sms", state: "ACCEPTED"}))?.outcome
        ).toBe("ACCEPTED")
    })

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
