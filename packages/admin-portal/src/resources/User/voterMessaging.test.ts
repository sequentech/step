// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import {
    EMessageAttemptState,
    EMessageChannel,
    ILedgerMessage,
    VOTER_ATTR_MESSAGE_CHANNEL,
    VOTER_ATTR_MESSAGE_NUMBER,
    VOTER_ATTR_MESSENGER_ID,
    VOTER_ATTR_MESSENGER_PAGE,
    VOTER_ATTR_MOBILE_NUMBER,
    VOTER_ATTR_VERIFIED_CHANNELS,
    VOTER_ATTR_VIBER_NUMBER,
    VOTER_ATTR_WHATSAPP_NUMBER,
} from "@/types/messaging"
import {
    EVoterChannelSource,
    IVoterChannel,
    isVoterMessagingAttribute,
    voterChannel,
    voterDeliveryProblems,
    voterMessaging,
} from "./voterMessaging"

// sequent-core's `voter_channel_tests.rs`, case for case (R7-05). sequent-core's
// WASM package doesn't export `voter_channel`, so the drawer mirrors it; every
// Rust test is a row here, and the guard below fails when one is added.
const RUST_TESTS = "sequent-core/src/types/voter_channel_tests.rs"
const PACKAGES = resolve(__dirname, "../../../..")

const MC = VOTER_ATTR_MESSAGE_CHANNEL
const MN = VOTER_ATTR_MESSAGE_NUMBER
const MOB = VOTER_ATTR_MOBILE_NUMBER
const MID = VOTER_ATTR_MESSENGER_ID
const MP = VOTER_ATTR_MESSENGER_PAGE
const VC = VOTER_ATTR_VERIFIED_CHANNELS
const WN = VOTER_ATTR_WHATSAPP_NUMBER
const VN = VOTER_ATTR_VIBER_NUMBER
const EMAIL = "voter@example.org"
const {RECORDED, VERIFIED_CHANNELS, LEGACY} = EVoterChannelSource
const {EMAIL: E, SMS, WHATSAPP, VIBER, MESSENGER} = EMessageChannel

interface IRustCase {
    /** The Rust test the row comes from. */
    rust: string
    email: string | null
    attributes?: Record<string, string[]>
    expected?: IVoterChannel
}

const RUST_CASES: IRustCase[] = [
    {
        rust: "recorded_phone_channel",
        email: EMAIL,
        attributes: {[MC]: ["VIBER"], [MN]: ["+639171234567"], [MOB]: ["+639170000000"]},
        expected: {channel: VIBER, destination: "+639171234567", source: RECORDED},
    },
    {
        rust: "recorded_messenger_needs_its_page",
        email: null,
        attributes: {[MC]: ["MESSENGER"], [MID]: ["psid-1"], [MP]: ["page-1"]},
        expected: {
            channel: MESSENGER,
            destination: "psid-1",
            messengerPage: "page-1",
            source: RECORDED,
        },
    },
    {
        rust: "recorded_messenger_needs_its_page",
        email: null,
        attributes: {[MC]: ["MESSENGER"], [MID]: ["psid-1"]},
        expected: undefined,
    },
    {
        rust: "first_release_record_prefers_its_verified_preference",
        email: EMAIL,
        attributes: {
            [MC]: ["VIBER"],
            [VC]: ["WHATSAPP", "VIBER"],
            [WN]: ["+15550000001"],
            [VN]: ["+15550000002"],
        },
        expected: {channel: VIBER, destination: "+15550000002", source: VERIFIED_CHANNELS},
    },
    {
        rust: "first_release_record_ignores_an_unverified_preference",
        email: EMAIL,
        attributes: {
            [MC]: ["EMAIL"],
            [VC]: ["SMS", "WHATSAPP"],
            [WN]: ["+15550000001"],
            [MOB]: ["+15550000003"],
        },
        expected: {channel: WHATSAPP, destination: "+15550000001", source: VERIFIED_CHANNELS},
    },
    {
        rust: "legacy_email_then_mobile",
        email: EMAIL,
        attributes: {[MOB]: ["+15550000003"]},
        expected: {channel: E, destination: EMAIL, source: LEGACY},
    },
    {
        rust: "legacy_email_then_mobile",
        email: "",
        attributes: {[MOB]: ["+15550000003"]},
        expected: {channel: SMS, destination: "+15550000003", source: LEGACY},
    },
    {rust: "legacy_email_then_mobile", email: null, attributes: undefined, expected: undefined},
    {
        rust: "recorded_channel_without_destination_has_no_channel",
        email: EMAIL,
        attributes: {[MC]: ["WHATSAPP"]},
        expected: undefined,
    },
    {
        rust: "recorded_channel_without_destination_has_no_channel",
        email: EMAIL,
        attributes: {[MC]: ["MESSENGER"], [MID]: ["psid-1"], [MOB]: ["+15550000003"]},
        expected: undefined,
    },
    {
        rust: "recorded_sms_reads_the_mobile_number_without_a_message_number",
        email: EMAIL,
        attributes: {[MC]: ["SMS"], [MOB]: ["+15550000003"]},
        expected: {channel: SMS, destination: "+15550000003", source: RECORDED},
    },
    {
        rust: "first_release_record_without_destination_has_no_channel",
        email: EMAIL,
        attributes: {[VC]: ["WHATSAPP"], [MOB]: ["+15550000003"]},
        expected: undefined,
    },
    {
        rust: "empty_verified_channels_is_not_a_first_release_record",
        email: EMAIL,
        attributes: {[VC]: []},
        expected: {channel: E, destination: EMAIL, source: LEGACY},
    },
    {
        rust: "channel_values_are_read_trimmed_and_in_any_case",
        email: EMAIL,
        attributes: {[MC]: ["whatsapp"], [MN]: ["+15550000001"]},
        expected: {channel: WHATSAPP, destination: "+15550000001", source: RECORDED},
    },
    {
        rust: "channel_values_are_read_trimmed_and_in_any_case",
        email: EMAIL,
        attributes: {[MC]: [" SMS "], [MOB]: ["+15550000003"]},
        expected: {channel: SMS, destination: "+15550000003", source: RECORDED},
    },
    {
        rust: "channel_values_are_read_trimmed_and_in_any_case",
        email: null,
        attributes: {
            [MC]: ["Viber"],
            [VC]: ["whatsapp", " Viber"],
            [WN]: ["+15550000001"],
            [VN]: ["+15550000002"],
        },
        expected: {channel: VIBER, destination: "+15550000002", source: VERIFIED_CHANNELS},
    },
]

describe("voterChannel: sequent-core's voter_channel cases (R7-05)", () => {
    it("has a row for every Rust test", () => {
        const text = readFileSync(resolve(PACKAGES, RUST_TESTS), "utf8")
        const rust = Array.from(text.matchAll(/#\[test\]\s*fn (\w+)/g), (match) => match[1])
        expect(rust.length).toBeGreaterThan(5)
        expect(Array.from(new Set(RUST_CASES.map((row) => row.rust))).sort()).toEqual(rust.sort())
    })

    it.each(RUST_CASES.map((row, index) => [`${row.rust} #${index}`, row] as const))(
        "%s",
        (_name, {email, attributes, expected}) => {
            expect(voterChannel(email, attributes)).toEqual(expected)
        }
    )
})

// How `voter_channel` reads values, beyond the Rust tests: the first value
// only, `str::trim` (Unicode White_Space, not U+FEFF) and ASCII case.
describe("voterChannel reads values as sequent-core does (R7-05)", () => {
    const legacy = {channel: E, destination: EMAIL, source: LEGACY}

    it("trims Unicode white space but not a byte order mark", () => {
        expect(voterChannel(EMAIL, {[MC]: ["\u00a0SMS\u3000"], [MOB]: ["+15550000003"]})).toEqual({
            channel: SMS,
            destination: "+15550000003",
            source: RECORDED,
        })
        expect(voterChannel(EMAIL, {[MC]: ["\ufeffSMS"], [MOB]: ["+15550000003"]})).toEqual(legacy)
    })

    it("reads ASCII case only", () => {
        expect(voterChannel(EMAIL, {[MC]: ["\u017fms"], [MOB]: ["+15550000003"]})).toEqual(legacy)
    })

    it("reads only the first value, and an empty one is none", () => {
        expect(voterChannel(EMAIL, {[MC]: ["", "SMS"], [MOB]: ["+15550000003"]})).toEqual(legacy)
        expect(voterChannel(EMAIL, {[MC]: ["SMS"], [MN]: ["", "+15550000001"]})).toBeUndefined()
    })

    it("takes a verified-channels list with no channel in it as a first-release record", () => {
        expect(voterChannel(EMAIL, {[VC]: [""]})).toBeUndefined()
        expect(voterChannel(EMAIL, {[VC]: ["BOGUS"]})).toBeUndefined()
    })

    it("keeps a value that isn't a channel out of it", () => {
        expect(voterChannel(undefined, {[MC]: ["NONE"]})).toBeUndefined()
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
