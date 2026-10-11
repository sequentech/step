// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import {EMessageChannel} from "@/types/messaging"
import {CALLING_CODES, callingCode, maskDestination} from "./maskDestination"

const PACKAGES = resolve(__dirname, "../../..")
const source = (path: string) => readFileSync(resolve(PACKAGES, path), "utf8")

const {EMAIL, SMS, WHATSAPP, VIBER, MESSENGER} = EMessageChannel
const hidden = "•".repeat(8)

interface IMaskCase {
    /** The test of `messaging/src/destination.rs` the row comes from. */
    rust: string
    channel: EMessageChannel
    raw: string
    /** `Destination::parse(channel, raw)?.masked()`; undefined when parse refuses it. */
    masked?: string
}

// `Destination::parse` then `Destination::masked`, on the Rust tests' own
// vectors (R7-11): the TS mask is the backend's, character for character.
const RUST_CASES: IMaskCase[] = [
    {
        rust: "phone_numbers_are_normalized_to_e164",
        channel: WHATSAPP,
        raw: "+63 (917) 123-4567",
        masked: "+63 ••• ••• 567",
    },
    {rust: "phone_numbers_without_a_country_code_are_rejected", channel: SMS, raw: "09171234567"},
    {rust: "phone_numbers_without_a_country_code_are_rejected", channel: SMS, raw: "+63abc"},
    {rust: "phone_numbers_without_a_country_code_are_rejected", channel: SMS, raw: "+1234"},
    {rust: "phone_numbers_without_a_country_code_are_rejected", channel: VIBER, raw: "+0123456789"},
    {
        rust: "calling_codes_are_matched_by_length",
        channel: SMS,
        raw: "+15550123456",
        masked: "+1 ••• ••• 456",
    },
    {
        rust: "calling_codes_are_matched_by_length",
        channel: SMS,
        raw: "+447700900123",
        masked: "+44 ••• ••• 123",
    },
    {
        rust: "calling_codes_are_matched_by_length",
        channel: SMS,
        raw: "+971501234567",
        masked: "+971 ••• ••• 567",
    },
    {
        rust: "emails_are_lowercased_and_masked",
        channel: EMAIL,
        raw: " Voter@Example.ORG ",
        masked: `vo${hidden}@example.org`,
    },
    {
        rust: "emails_are_lowercased_and_masked",
        channel: EMAIL,
        raw: "ab@example.org",
        masked: `a${hidden}@example.org`,
    },
    {
        rust: "emails_are_lowercased_and_masked",
        channel: EMAIL,
        raw: "a@example.org",
        masked: `${hidden}@example.org`,
    },
    {rust: "emails_are_lowercased_and_masked", channel: EMAIL, raw: "voter"},
    {rust: "emails_are_lowercased_and_masked", channel: EMAIL, raw: "a b@example.org"},
    {
        rust: "messenger_recipients_are_page_scoped_ids",
        channel: MESSENGER,
        raw: "6543210987654321",
        masked: "••• 321",
    },
    {rust: "messenger_recipients_are_page_scoped_ids", channel: MESSENGER, raw: "+639171234567"},
]

// Rust tests with no masking in them.
const NOT_MASKING = ["digests_depend_on_the_key_and_not_on_formatting"]

describe("maskDestination: messaging's Destination::masked (R7-11)", () => {
    it("has a row for every Rust test", () => {
        const text = source("messaging/src/destination.rs")
        const tests = text.slice(text.indexOf("mod tests"))
        const rust = Array.from(tests.matchAll(/#\[test\]\s*fn (\w+)/g), (match) => match[1])
        expect(rust.length).toBeGreaterThan(4)
        expect(
            [...Array.from(new Set(RUST_CASES.map((row) => row.rust))), ...NOT_MASKING].sort()
        ).toEqual(rust.sort())
    })

    it.each(RUST_CASES.map((row) => [`${row.rust}: ${JSON.stringify(row.raw)}`, row] as const))(
        "%s",
        (_name, {channel, raw, masked}) => {
            expect(maskDestination(channel, raw)).toBe(masked)
        }
    )

    it("hides the same number of characters whatever the address's length", () => {
        expect(maskDestination(SMS, "+63917123456789")).toBe("+63 ••• ••• 789")
        expect(maskDestination(EMAIL, "a.very.long.local.part@example.org")).toBe(
            `a.${hidden}@example.org`
        )
    })

    it("counts an email's characters, not its UTF-16 units", () => {
        expect(maskDestination(EMAIL, "\u{1F600}x@example.org")).toBe(
            `\u{1F600}${hidden}@example.org`
        )
    })
})

describe("callingCode", () => {
    it("matches calling codes by length, as dial_codes::calling_code", () => {
        expect(callingCode("+15550123456")).toBe("1")
        expect(callingCode("+447700900123")).toBe("44")
        expect(callingCode("+971501234567")).toBe("971")
        expect(callingCode("+0123456789")).toBeUndefined()
        // Like the Rust, a number without its + has no calling code.
        expect(callingCode("447700900123")).toBeUndefined()
    })

    it("knows the same calling codes as sequent-core's dial_codes, global services included", () => {
        const text = source("sequent-core/src/types/dial_codes.rs")
        const table = (name: string, end: string) =>
            text.slice(
                text.indexOf(`pub const ${name}`),
                text.indexOf(end, text.indexOf(`pub const ${name}`))
            )
        const geographic = Array.from(
            table("CALLING_CODES", "\n];").matchAll(/\("(\d+)"/g),
            (match) => match[1]
        )
        const global = Array.from(
            table("NON_GEOGRAPHIC_CODES", "];").matchAll(/"(\d+)"/g),
            (match) => match[1]
        )
        expect(geographic.length).toBeGreaterThan(200)
        expect(global).toContain("800")
        expect(Array.from(CALLING_CODES).sort()).toEqual(
            Array.from(new Set([...geographic, ...global])).sort()
        )
    })
})
