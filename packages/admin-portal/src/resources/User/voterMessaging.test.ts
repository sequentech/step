// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageAttemptState, EMessageChannel, ILedgerMessage} from "@/types/messaging"
import {
    EVoterChannelSource,
    isVoterMessagingAttribute,
    voterChannel,
    voterDeliveryProblems,
    voterMessaging,
} from "./voterMessaging"

// The same cases as sequent-core's voter_channel tests and Keycloak's VoterChannel.
describe("voterChannel", () => {
    it("reads the recorded channel and its number", () => {
        expect(
            voterChannel("voter@example.org", {
                "sequent.read-only.message-channel": ["WHATSAPP"],
                "sequent.read-only.message-number": ["+34600111214"],
            })
        ).toEqual({
            channel: EMessageChannel.WHATSAPP,
            destination: "+34600111214",
            source: EVoterChannelSource.RECORDED,
        })
    })

    it("reads Messenger only with both the Page-scoped ID and the Page", () => {
        expect(
            voterChannel(null, {
                "sequent.read-only.message-channel": ["MESSENGER"],
                "sequent.read-only.messenger-id": ["7304419925518842"],
                "sequent.read-only.messenger-page": ["118204557331906"],
            })
        ).toEqual({
            channel: EMessageChannel.MESSENGER,
            destination: "7304419925518842",
            messengerPage: "118204557331906",
            source: EVoterChannelSource.RECORDED,
        })
        expect(
            voterChannel("voter@example.org", {
                "sequent.read-only.message-channel": ["MESSENGER"],
                "sequent.read-only.messenger-id": ["7304419925518842"],
            })?.channel
        ).toBe(EMessageChannel.EMAIL)
    })

    it("reads a first-release record: its channel if verified, else the first in channel order", () => {
        const record = {
            "sequent.read-only.message-channel": ["SMS"],
            "sequent.read-only.verified-channels": ["EMAIL", "VIBER", "BOGUS"],
            "sequent.read-only.viber-number": ["+971552345678"],
        }
        expect(voterChannel("voter@example.org", record)).toEqual({
            channel: EMessageChannel.VIBER,
            destination: "+971552345678",
            source: EVoterChannelSource.VERIFIED_CHANNELS,
        })
        expect(
            voterChannel("voter@example.org", {
                ...record,
                "sequent.read-only.message-channel": ["EMAIL"],
            })?.channel
        ).toBe(EMessageChannel.EMAIL)
    })

    it("gives a legacy voter the email, else the mobile number", () => {
        expect(voterChannel("voter@example.org", {})?.channel).toBe(EMessageChannel.EMAIL)
        expect(voterChannel("", {"sequent.read-only.mobile-number": ["+639171234567"]})).toEqual({
            channel: EMessageChannel.SMS,
            destination: "+639171234567",
            source: EVoterChannelSource.LEGACY,
        })
    })

    it("has no channel without any destination, and ignores reset values", () => {
        expect(voterChannel(undefined, undefined)).toBeUndefined()
        expect(
            voterChannel(undefined, {"sequent.read-only.message-channel": ["NONE"]})
        ).toBeUndefined()
    })
})

describe("voterMessaging", () => {
    it("reads the consent time", () => {
        const messaging = voterMessaging("voter@example.org", {
            "sequent.read-only.message-consent": [
                JSON.stringify({
                    at: "2026-10-01T08:30:00Z",
                    version: "1",
                    organization: "Staff Association",
                    channel: "EMAIL",
                }),
            ],
        })
        expect(messaging.consentAt).toBe("2026-10-01T08:30:00Z")
        expect(
            voterMessaging(null, {"sequent.read-only.message-consent": ["{bad"]}).consentAt
        ).toBe(undefined)
    })

    it("recognises the messaging attributes, old names included", () => {
        expect(isVoterMessagingAttribute("sequent.read-only.message-number")).toBe(true)
        expect(isVoterMessagingAttribute("sequent.read-only.viber-number")).toBe(true)
        expect(isVoterMessagingAttribute("sequent.read-only.mobile-number")).toBe(false)
    })
})

describe("voterDeliveryProblems (R4-07)", () => {
    const attempt = (overrides: Partial<ILedgerMessage>): ILedgerMessage => ({
        id: `a-${overrides.attempt ?? 1}-${overrides.logical_key ?? "k"}`,
        channel: EMessageChannel.VIBER,
        state: EMessageAttemptState.FAILED,
        attempt: 1,
        logical_key: "k",
        created_at: "2026-10-09T10:00:00Z",
        ...overrides,
    })

    it("judges a message by its latest attempt: a retry in progress is not a failure", () => {
        const failed = attempt({attempt: 1})
        const retry = attempt({
            attempt: 2,
            state: EMessageAttemptState.QUEUED,
            created_at: "2026-10-09T10:01:00Z",
        })
        expect(voterDeliveryProblems([failed], [failed, retry])).toEqual({
            failure: undefined,
            unconfirmed: undefined,
        })
    })

    it("keeps the latest failed message, and an unconfirmed one apart", () => {
        const failed = attempt({attempt: 3, logical_key: "notice"})
        const unknown = attempt({
            attempt: 1,
            logical_key: "otp",
            state: EMessageAttemptState.UNKNOWN,
        })
        expect(voterDeliveryProblems([unknown, failed], [unknown, failed])).toEqual({
            failure: failed,
            unconfirmed: unknown,
        })
    })
})
