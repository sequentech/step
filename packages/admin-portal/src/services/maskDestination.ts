// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EMessageChannel} from "@/types/messaging"
import {hasRustWhitespace, rustTrim} from "./rustText"

/**
 * Country calling codes (ITU-T E.164), as in
 * packages/sequent-core/src/types/dial_codes.rs (`CALLING_CODES` and the
 * global services' `NON_GEOGRAPHIC_CODES`). They are prefix-free, so the first
 * match by length is the code. A test keeps the two lists the same.
 */
export const CALLING_CODES: ReadonlySet<string> = new Set([
    "1",
    "7",
    "20",
    "27",
    "30",
    "31",
    "32",
    "33",
    "34",
    "36",
    "39",
    "40",
    "41",
    "43",
    "44",
    "45",
    "46",
    "47",
    "48",
    "49",
    "51",
    "52",
    "53",
    "54",
    "55",
    "56",
    "57",
    "58",
    "60",
    "61",
    "62",
    "63",
    "64",
    "65",
    "66",
    "81",
    "82",
    "84",
    "86",
    "90",
    "91",
    "92",
    "93",
    "94",
    "95",
    "98",
    "211",
    "212",
    "213",
    "216",
    "218",
    "220",
    "221",
    "222",
    "223",
    "224",
    "225",
    "226",
    "227",
    "228",
    "229",
    "230",
    "231",
    "232",
    "233",
    "234",
    "235",
    "236",
    "237",
    "238",
    "239",
    "240",
    "241",
    "242",
    "243",
    "244",
    "245",
    "246",
    "247",
    "248",
    "249",
    "250",
    "251",
    "252",
    "253",
    "254",
    "255",
    "256",
    "257",
    "258",
    "260",
    "261",
    "262",
    "263",
    "264",
    "265",
    "266",
    "267",
    "268",
    "269",
    "290",
    "291",
    "297",
    "298",
    "299",
    "350",
    "351",
    "352",
    "353",
    "354",
    "355",
    "356",
    "357",
    "358",
    "359",
    "370",
    "371",
    "372",
    "373",
    "374",
    "375",
    "376",
    "377",
    "378",
    "379",
    "380",
    "381",
    "382",
    "383",
    "385",
    "386",
    "387",
    "389",
    "420",
    "421",
    "423",
    "500",
    "501",
    "502",
    "503",
    "504",
    "505",
    "506",
    "507",
    "508",
    "509",
    "590",
    "591",
    "592",
    "593",
    "594",
    "595",
    "596",
    "597",
    "598",
    "599",
    "670",
    "672",
    "673",
    "674",
    "675",
    "676",
    "677",
    "678",
    "679",
    "680",
    "681",
    "682",
    "683",
    "685",
    "686",
    "687",
    "688",
    "689",
    "690",
    "691",
    "692",
    "850",
    "852",
    "853",
    "855",
    "856",
    "870",
    "880",
    "886",
    "960",
    "961",
    "962",
    "963",
    "964",
    "965",
    "966",
    "967",
    "968",
    "970",
    "971",
    "972",
    "973",
    "974",
    "975",
    "976",
    "977",
    "992",
    "993",
    "994",
    "995",
    "996",
    "998",
    // Global services: valid numbers with no country.
    "800",
    "808",
    "878",
    "881",
    "882",
    "883",
    "888",
    "979",
])

const HIDDEN = "\u2022\u2022\u2022"

/** The country calling code of an E.164 number (`+` and digits), as digits: `dial_codes::calling_code`. */
export const callingCode = (e164: string): string | undefined => {
    if (!e164.startsWith("+")) return undefined
    const digits = e164.slice(1)
    return [1, 2, 3]
        .filter((length) => digits.length >= length)
        .map((length) => digits.slice(0, length))
        .find((code) => CALLING_CODES.has(code))
}

/** `normalize_phone`: E.164 once spaces, dashes, brackets and dots are removed. */
const normalizePhone = (raw: string): string | undefined => {
    const compact = raw.replace(/[ \-().]/g, "")
    if (!compact.startsWith("+")) return undefined
    const digits = compact.slice(1)
    if (!/^[0-9]{8,15}$/.test(digits)) return undefined
    const normalized = `+${digits}`
    return callingCode(normalized) ? normalized : undefined
}

/** `normalize_email`: trimmed and lowercased; a local part, and a domain with a dot. */
const normalizeEmail = (raw: string): string | undefined => {
    const email = rustTrim(raw).toLowerCase()
    const at = email.indexOf("@")
    if (at < 0) return undefined
    const local = email.slice(0, at)
    const domain = email.slice(at + 1)
    return local !== "" &&
        domain.includes(".") &&
        !domain.startsWith(".") &&
        !hasRustWhitespace(email)
        ? email
        : undefined
}

/** `normalize_page_scoped_id`: digits only. */
const normalizePageScopedId = (raw: string): string | undefined => {
    const id = rustTrim(raw)
    return /^[0-9]+$/.test(id) ? id : undefined
}

const lastThree = (value: string) => value.slice(Math.max(0, value.length - 3))

/**
 * A destination as screens and records show it: the backend's
 * `Destination::parse(channel, raw)` then `Destination::masked`
 * (`packages/messaging/src/destination.rs`), on the same test vectors (R7-11).
 * - a number: its country code and last 3 digits (`+966 ••• ••• 567`);
 * - an email: its first 2 characters, fewer for a local part of 1 or 2 and
 *   never all of it, and its domain (`ma••••••••@example.com`);
 * - a Page-scoped ID: its last 3 digits (`••• 321`).
 *
 * The masked parts have a fixed length, so they don't reveal the address's
 * length. Undefined when the backend refuses the destination: it would not
 * send to it ("Invalid destination for the channel").
 */
export const maskDestination = (channel: EMessageChannel, raw: string): string | undefined => {
    switch (channel) {
        case EMessageChannel.SMS:
        case EMessageChannel.WHATSAPP:
        case EMessageChannel.VIBER: {
            const phone = normalizePhone(raw)
            if (!phone) return undefined
            const code = callingCode(phone) ?? ""
            return `+${code} ${HIDDEN} ${HIDDEN} ${lastThree(phone.slice(1 + code.length))}`
        }
        case EMessageChannel.EMAIL: {
            const email = normalizeEmail(raw)
            if (!email) return undefined
            const at = email.indexOf("@")
            // Characters, as Rust's `chars()`: never half of a surrogate pair.
            const local = Array.from(email.slice(0, at))
            const shown = Math.min(2, Math.max(0, local.length - 1))
            return `${local.slice(0, shown).join("")}${HIDDEN}${HIDDEN}\u2022\u2022${email.slice(at)}`
        }
        case EMessageChannel.MESSENGER: {
            const id = normalizePageScopedId(raw)
            return id ? `${HIDDEN} ${lastThree(id)}` : undefined
        }
    }
}
