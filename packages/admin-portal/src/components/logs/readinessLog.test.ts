// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createInstance} from "i18next"
import en from "@/translations/en"
import {readinessCheckLines, readinessDescription, readinessDetailRows} from "./readinessLog"
import type {ILogMessage} from "./logMessage"

const message = (details: unknown): ILogMessage => ({
    statement: {body: {Audit: {details_json: JSON.stringify(details)}}},
})

async function translate(overrides: Record<string, string> = {}) {
    const i18n = createInstance()
    await i18n.init({
        lng: "en",
        resources: {en: {translation: structuredClone(en.translations)}},
        interpolation: {escapeValue: false},
    })
    for (const [key, value] of Object.entries(overrides))
        i18n.addResource("en", "translation", key, value)
    return i18n.t.bind(i18n)
}

it("translates current and legacy receipt area counts without changing signed details", async () => {
    const t = await translate()
    for (const key of ["counts_by_area", "counts_by_post"]) {
        const signed = message({import_id: "import-1", [key]: {"Area A": 12, "Area B": 0}})
        const original = JSON.stringify(signed)
        expect(readinessDetailRows(t, "VotersImported", signed)).toEqual([
            {name: "Import ID", value: "import-1"},
            {name: "Voters by area", value: "Area A: 12; Area B: 0"},
        ])
        expect(JSON.stringify(signed)).toBe(original)
    }
})

it("uses stable check kinds and tenant label overrides for legacy signed reports", async () => {
    const t = await translate({
        "logsScreen.readiness.fields.counts_by_area": "Voters by district",
        "logsScreen.readiness.fields.areas": "Districts",
        "logsScreen.readiness.checks.certified_voters":
            "Both events match the certified district registry.",
    })
    const signed = message({
        report: {
            checks: [
                {kind: "certified_voters", passed: true, message: "Legacy English counts per Post"},
            ],
            evidence: {
                official: {
                    configuration: {posts: ["Area A"]},
                    voters: {counts_by_post: {"Area A": 12}},
                },
            },
        },
    })
    const original = JSON.stringify(signed)
    const rows = readinessDetailRows(t, "ReadinessVerified", signed)
    expect(rows?.[0].value).toContain("Districts: Area A")
    expect(rows?.[0].value).toContain("Voters by district: Area A: 12")
    expect(rows?.[0].value).not.toContain("counts_by_post")
    expect(rows?.[0].value).not.toContain("Legacy English")
    expect(readinessCheckLines(t, signed)).toEqual([
        "Passed: Both events match the certified district registry.",
    ])
    expect(JSON.stringify(signed)).toBe(original)
})

it("provides neutral translated descriptions and leaves unrelated statement kinds alone", async () => {
    const t = await translate()
    expect(readinessDescription(t, "VotersImported")).toBe("Voter file imported.")
    expect(readinessDescription(t, "ReadinessVerified")).toBe(
        "Official event readiness verified against the tested event."
    )
    expect(readinessDescription(t, "AdminChange")).toBeNull()
    expect(readinessDetailRows(t, "AdminChange", message({report: {passed: true}}))).toBeNull()
})

it("preserves every dynamic area name even when it resembles a structural field or alias", async () => {
    const t = await translate()
    const counts = {
        checks: 1,
        posts: 2,
        areas: 3,
        constructor: 4,
        counts_by_post: 5,
        counts_by_area: 6,
    }
    for (const field of ["counts_by_area", "counts_by_post", "current_voter_counts"]) {
        const signed = message({[field]: counts})
        const before = JSON.stringify(signed)
        expect(readinessDetailRows(t, "VotersImported", signed)?.[0].value).toBe(
            "checks: 1; posts: 2; areas: 3; constructor: 4; counts_by_post: 5; counts_by_area: 6"
        )
        expect(JSON.stringify(signed)).toBe(before)
    }
})

it("shows every contributing publication with translated labels alongside the legacy reference", async () => {
    const t = await translate()
    const signed = message({
        report: {
            evidence: {
                official: {
                    publication: {
                        publication_id: "latest",
                        publication_ids: ["latest", "earlier"],
                    },
                },
            },
        },
    })
    const original = JSON.stringify(signed)
    const rows = readinessDetailRows(t, "ReadinessVerified", signed)
    expect(rows?.[0].value).toContain("Publication ID: latest")
    expect(rows?.[0].value).toContain("Publication IDs: latest, earlier")
    expect(JSON.stringify(signed)).toBe(original)
})
