// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    clearVotingPortalDateTimeCache,
    DateTimePatternError,
    formatVotingPortalDateTime,
    isValidVotingPortalDateTimePattern,
    parseVotingPortalDateTimePattern,
    VOTING_PORTAL_DATETIME_FORMAT_KEY,
    VotingPortalDateTimeEvent,
} from "./votingPortalDateTime"
import {
    EVotingPortalDateTimeFormat,
    IElectionEventPresentation,
} from "../types/ElectionEventPresentation"
import {ETranslationScope} from "./translationScopes"

// Built with the local-time constructor so assertions are timezone-independent:
// both construction and (default) Intl formatting use the local timezone.
const FIXED_DATE = new Date(2026, 2, 9, 7, 5, 30) // 2026-03-09 07:05:30 local

const makeEvent = (
    id: string,
    presentation: Partial<IElectionEventPresentation>
): VotingPortalDateTimeEvent => ({id, presentation: presentation as IElectionEventPresentation})

beforeEach(() => {
    clearVotingPortalDateTimeCache()
})

describe("parseVotingPortalDateTimePattern", () => {
    it("formats a valid token pattern", () => {
        expect(parseVotingPortalDateTimePattern("dd/MM/yyyy HH:mm")(FIXED_DATE)).toBe(
            "09/03/2026 07:05"
        )
        expect(parseVotingPortalDateTimePattern("yyyy-MM-dd")(FIXED_DATE)).toBe("2026-03-09")
        expect(parseVotingPortalDateTimePattern("HH:mm:ss")(FIXED_DATE)).toBe("07:05:30")
    })

    it("passes through literal characters", () => {
        expect(parseVotingPortalDateTimePattern("yyyy.MM.dd")(FIXED_DATE)).toBe("2026.03.09")
    })

    it("throws on an empty pattern", () => {
        expect(() => parseVotingPortalDateTimePattern("")).toThrow(DateTimePatternError)
        expect(() => parseVotingPortalDateTimePattern("   ")).toThrow(DateTimePatternError)
    })

    it("throws on a pattern with no recognized token", () => {
        expect(() => parseVotingPortalDateTimePattern("hello world")).toThrow(DateTimePatternError)
    })

    it("rejects misused tokens from other conventions (YYYY, DD, hh)", () => {
        expect(() => parseVotingPortalDateTimePattern("DD/MM/YYYY")).toThrow(DateTimePatternError)
        expect(() => parseVotingPortalDateTimePattern("dd/MM/YYYY")).toThrow(DateTimePatternError)
        expect(() => parseVotingPortalDateTimePattern("DD/MM/yyyy")).toThrow(DateTimePatternError)
        expect(() => parseVotingPortalDateTimePattern("yyyy-MM-dd hh:mm")).toThrow(
            DateTimePatternError
        )
    })
})

describe("isValidVotingPortalDateTimePattern", () => {
    it("returns true for a pattern with at least one token", () => {
        expect(isValidVotingPortalDateTimePattern("dd/MM/yyyy")).toBe(true)
    })

    it("returns false for empty, whitespace, or tokenless patterns", () => {
        expect(isValidVotingPortalDateTimePattern("")).toBe(false)
        expect(isValidVotingPortalDateTimePattern("   ")).toBe(false)
        expect(isValidVotingPortalDateTimePattern("hello world")).toBe(false)
    })

    it("returns false for patterns with misused tokens", () => {
        expect(isValidVotingPortalDateTimePattern("DD/MM/YYYY")).toBe(false)
        expect(isValidVotingPortalDateTimePattern("hh:mm")).toBe(false)
    })
})

describe("formatVotingPortalDateTime — presets", () => {
    it("defaults to the legacy GB 24h format when the field is absent", () => {
        const out = formatVotingPortalDateTime(FIXED_DATE, makeEvent("e-absent", {}), "en")
        expect(out).toContain("09/03/2026")
        expect(out).toContain("07:05")
    })

    it("defaults to legacy when the event/presentation is missing entirely", () => {
        const out = formatVotingPortalDateTime(FIXED_DATE, null, "en")
        expect(out).toContain("09/03/2026")
        expect(out).toContain("07:05")
    })

    it("renders ISO_LOCAL", () => {
        const event = makeEvent("e-iso", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
        })
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("2026-03-09 07:05")
    })

    it("renders US_12H", () => {
        const event = makeEvent("e-us", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.US_12H,
        })
        const out = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        expect(out).toContain("03/09/2026")
        expect(out).toMatch(/7:05/)
        expect(out).toMatch(/AM/i)
    })

    it("renders LOCALE_MEDIUM with a time component", () => {
        const event = makeEvent("e-locale", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.LOCALE_MEDIUM,
        })
        const out = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        expect(out).toContain("2026")
        expect(out).toMatch(/\d{1,2}:\d{2}/)
    })

    it("renders DATE_ONLY with no time component", () => {
        const event = makeEvent("e-date", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.DATE_ONLY,
        })
        const out = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        expect(out).toContain("2026")
        expect(out).not.toContain(":")
    })

    it("accepts epoch milliseconds and ISO strings", () => {
        const event = makeEvent("e-iso2", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
        })
        const fromMs = formatVotingPortalDateTime(FIXED_DATE.getTime(), event, "en")
        expect(fromMs).toBe("2026-03-09 07:05")
    })
})

describe("formatVotingPortalDateTime — custom event-level format", () => {
    it("renders the inline custom pattern", () => {
        const event = makeEvent("e-custom", {
            voting_portal_datetime_format: {custom: "dd/MM/yyyy HH:mm"},
        })
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("09/03/2026 07:05")
    })

    it("lets a per-language override take precedence over the custom format", () => {
        const event = makeEvent("e-custom-ov", {
            voting_portal_datetime_format: {custom: "dd/MM/yyyy HH:mm"},
            i18n: {en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: "yyyy-MM-dd"}},
        })
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("2026-03-09")
    })

    it("falls back to the legacy format on an invalid custom pattern", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        const event = makeEvent("e-custom-bad", {
            voting_portal_datetime_format: {custom: "no tokens here"},
        })
        const out = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        expect(out).toContain("09/03/2026")
        expect(out).toContain("07:05")
        expect(warn).toHaveBeenCalledTimes(1)
        warn.mockRestore()
    })
})

describe("formatVotingPortalDateTime — per-language override", () => {
    const overrideEvent = (id: string) =>
        makeEvent(id, {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.US_12H,
            i18n: {
                en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: "yyyy-MM-dd"},
            },
        })

    it("uses the override for the matching language", () => {
        expect(formatVotingPortalDateTime(FIXED_DATE, overrideEvent("e-ov-1"), "en")).toBe(
            "2026-03-09"
        )
    })

    it("uses an explicitly scoped Voting Portal override", () => {
        const event = makeEvent("e-ov-scoped", {
            i18n: {
                en: {
                    [`${ETranslationScope.VOTING_PORTAL}:${VOTING_PORTAL_DATETIME_FORMAT_KEY}`]:
                        "yyyy-MM-dd",
                },
            },
        })

        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("2026-03-09")
    })

    it("prefers a Voting Portal override over legacy and global values", () => {
        const event = makeEvent("e-ov-precedence", {
            i18n: {
                en: {
                    [`${ETranslationScope.GLOBAL}:${VOTING_PORTAL_DATETIME_FORMAT_KEY}`]:
                        "MM/dd/yyyy",
                    [VOTING_PORTAL_DATETIME_FORMAT_KEY]: "dd/MM/yyyy",
                    [`${ETranslationScope.VOTING_PORTAL}:${VOTING_PORTAL_DATETIME_FORMAT_KEY}`]:
                        "yyyy-MM-dd",
                },
            },
        })

        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("2026-03-09")
    })

    it("falls through to the preset for languages without an override", () => {
        const out = formatVotingPortalDateTime(FIXED_DATE, overrideEvent("e-ov-2"), "es")
        // US_12H preset, not the en override
        expect(out).toContain("03/09/2026")
        expect(out).not.toBe("2026-03-09")
    })

    it("logs and falls back to the preset on a malformed override", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        const event = makeEvent("e-ov-bad", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
            i18n: {en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: "not a pattern"}},
        })
        const out = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        expect(out).toBe("2026-03-09 07:05") // ISO_LOCAL preset
        expect(warn).toHaveBeenCalledTimes(1)
        warn.mockRestore()
    })
})

describe("formatVotingPortalDateTime — override edge cases (FR5, FR7)", () => {
    it("falls through to the preset for an empty override without warning", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        const event = makeEvent("e-ov-empty", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
            i18n: {en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: ""}},
        })
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("2026-03-09 07:05")
        expect(warn).not.toHaveBeenCalled()
        warn.mockRestore()
    })

    it("warns and falls back to the preset for a whitespace-only override", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        const event = makeEvent("e-ov-ws", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
            i18n: {en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: "   "}},
        })
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("2026-03-09 07:05")
        expect(warn).toHaveBeenCalledTimes(1)
        warn.mockRestore()
    })

    it("never throws to the caller on an unrecognized override; resolves to the preset", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        const event = makeEvent("e-ov-bad-2", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.US_12H,
            i18n: {en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: "no tokens here"}},
        })
        let out = ""
        expect(() => {
            out = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        }).not.toThrow()
        expect(out).toContain("03/09/2026") // US_12H preset
        warn.mockRestore()
    })

    it("isolates the override to its language; another language uses the preset", () => {
        const event = makeEvent("e-ov-iso-lang", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
            i18n: {en: {[VOTING_PORTAL_DATETIME_FORMAT_KEY]: "dd/MM/yyyy"}},
        })
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "en")).toBe("09/03/2026")
        expect(formatVotingPortalDateTime(FIXED_DATE, event, "es")).toBe("2026-03-09 07:05")
    })
})

describe("formatVotingPortalDateTime — memoization", () => {
    it("returns a stable result across repeated calls", () => {
        const event = makeEvent("e-memo", {
            voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL,
        })
        const first = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        const second = formatVotingPortalDateTime(FIXED_DATE, event, "en")
        expect(first).toBe(second)
        expect(first).toBe("2026-03-09 07:05")
    })
})
