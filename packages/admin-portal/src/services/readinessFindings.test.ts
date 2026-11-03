// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createInstance} from "i18next"
import cat from "@/translations/cat"
import en from "@/translations/en"
import es from "@/translations/es"
import eu from "@/translations/eu"
import fr from "@/translations/fr"
import gl from "@/translations/gl"
import nl from "@/translations/nl"
import tl from "@/translations/tl"
import {
    findingText,
    READINESS_CERTIFICATION_REASONS,
    READINESS_FINDING_CODES,
    type IReadinessFinding,
} from "./readinessFindings"

const languages = {cat, en, es, eu, fr, gl, nl, tl}

async function translate(language: keyof typeof languages = "en") {
    const i18n = createInstance()
    await i18n.init({
        lng: language,
        resources: {[language]: {translation: structuredClone(languages[language].translations)}},
    })
    return i18n.t.bind(i18n)
}

const finding = (overrides: Partial<IReadinessFinding>): IReadinessFinding => ({
    code: "registry_count_changed",
    event: "official",
    message: "The official event's registry has 4 voters in area Area A",
    ...overrides,
})

it("explains a failure with its area, event and observed values", async () => {
    const t = await translate()
    expect(findingText(t, finding({area: "Area A", values: {imported: "3", current: "4"}}))).toBe(
        "The official event now has 4 voters in Area A, but its certified import has 3."
    )
    expect(
        findingText(
            t,
            finding({
                code: "voting_started",
                election_id: "election-1",
                values: {channel: "ONLINE", status: "OPEN"},
            })
        )
    ).toBe("Election election-1: the ONLINE channel is OPEN.")
    expect(
        findingText(
            t,
            finding({
                code: "voting_started",
                values: {channel: "KIOSK", status: "NOT_STARTED", dates: "recorded"},
            })
        )
    ).toBe("The official event: the KIOSK channel has recorded voting dates.")
    expect(
        findingText(
            t,
            finding({
                code: "voter_file_not_certified",
                event: "tested",
                values: {reason: "declared_sha256_missing"},
            })
        )
    ).toBe("The tested event’s last voter import is not certified: no declared SHA-256 was given.")
    expect(
        findingText(t, finding({code: "configuration_incomplete", values: {field: "approvers"}}))
    ).toBe("The official event’s signed configuration has no valid value for “Verified approvers”.")
    expect(findingText(t, finding({area: "", values: {imported: "0", current: "2"}}))).toBe(
        "The official event now has 2 voters in no or an unknown area, but its certified import has 0."
    )
})

it("keeps the server's own words for codes this portal does not know", async () => {
    const t = await translate()
    expect(findingText(t, finding({code: "future_check", message: "Server detail"}))).toBe(
        "Server detail"
    )
    expect(findingText(t, finding({code: "future_check", message: ""}))).toBe("future_check")
})

it("renders data from the evidence as text, never as markup", async () => {
    const t = await translate()
    expect(
        findingText(t, finding({area: "<b>A & B</b>", values: {imported: "1", current: "2"}}))
    ).toContain("<b>A & B</b>")
})

it.each(Object.keys(languages) as Array<keyof typeof languages>)(
    "explains every failure code and certification reason in %s",
    async (language) => {
        const readiness = languages[language].translations.readiness as unknown as Record<
            string,
            Record<string, string>
        >
        for (const code of READINESS_FINDING_CODES)
            expect(readiness.findings[code]?.trim()).toBeTruthy()
        for (const reason of READINESS_CERTIFICATION_REASONS)
            expect(readiness.certification[reason]?.trim()).toBeTruthy()
        for (const key of Object.keys(en.translations.readiness))
            expect(readiness).toHaveProperty(key)
        const t = await translate(language)
        expect(
            findingText(t, finding({area: "Area A", values: {imported: "3", current: "4"}}))
        ).toMatch(/Area A.*|.*Area A/)
    }
)
