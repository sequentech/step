/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, cleanup, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ReadinessCheckButton} from "./ReadinessCheckButton"
import {IPermissions} from "@/types/keycloak"

const mockRun = jest.fn()
const mockDownload = jest.fn()
const mockQuery = jest.fn()
const mockRefresh = jest.fn()
let mockAuthorized = true
let mockLanguage = "en"
const official = "22222222-2222-4222-8222-222222222222"
const tested = "33333333-3333-4333-8333-333333333333"
const other = "44444444-4444-4444-8444-444444444444"
const event = (id: string, name: string, archived = false) => ({
    id,
    presentation: {i18n: {en: {name}}},
    is_archived: archived,
})
const mockEvents = [
    event(official, "Official"),
    event(tested, "Rehearsal", true),
    event(other, "Other rehearsal"),
]
const report = (passed = true) => ({
    passed,
    checked_at: "2026-10-16T09:00:00Z",
    checks: ["configuration", "certified_voters", "official_zero", "tested_finished"].map(
        (kind) => ({
            kind,
            passed: passed || kind !== "certified_voters",
            message: "Server explanation",
        })
    ),
    evidence: {official: {event_id: official}, tested: {event_id: tested}},
    log_entry_id: passed ? "signed-entry-1" : null,
})

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockRun],
    useQuery: (...args: unknown[]) => mockQuery(...args),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: mockLanguage}}),
}))
jest.mock("react-admin", () => ({useRefresh: () => mockRefresh}))
jest.mock("@sequentech/ui-core", () => ({
    downloadBlob: (...args: unknown[]) => mockDownload(...args),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => mockAuthorized}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (value: {presentation: {i18n: {en: {name: string}}}}) =>
        value.presentation.i18n.en.name,
}))

beforeEach(() => {
    jest.clearAllMocks()
    mockAuthorized = true
    mockLanguage = "en"
    mockQuery.mockReturnValue({
        data: {sequent_backend_election_event: mockEvents},
        loading: false,
        refetch: jest.fn(),
    })
    mockRun.mockResolvedValue({data: {readiness_check: report()}})
    mockDownload.mockResolvedValue(undefined)
})

function open() {
    render(<ReadinessCheckButton event={event(official, "Official")} />)
    fireEvent.click(screen.getByRole("button", {name: "readiness.action"}))
}
const comparisonControl = () =>
    screen.getByRole("combobox", {name: /^readiness\.rehearsal\.label/})
function select(name = "Rehearsal", label = "readiness.testedEvent") {
    fireEvent.change(screen.getByRole("combobox", {name: label}), {
        target: {value: name},
    })
    fireEvent.click(screen.getByRole("option", {name: new RegExp(name)}))
}
async function run() {
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    await screen.findByText("signed-entry-1")
}

it("does not expose the action or fetch events without readiness permission", () => {
    mockAuthorized = false
    render(<ReadinessCheckButton event={event(official, "Official")} />)
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
    expect(mockQuery).not.toHaveBeenCalled()
})
it("requires a different tested event, including archived rehearsals", () => {
    open()
    expect(screen.getByRole("button", {name: "readiness.run"})).toBeDisabled()
    select()
    expect(screen.getByRole("button", {name: "readiness.run"})).toBeEnabled()
    expect(mockQuery.mock.calls[0][1]).toMatchObject({
        variables: {tenantId: "tenant-1", officialEventId: official},
        context: {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ}},
    })
})
it("runs as readiness-check and displays and downloads the signed report", async () => {
    open()
    select()
    await run()
    expect(mockRun).toHaveBeenCalledWith({
        variables: {official_event_id: official, tested_event_id: tested},
        context: {headers: {"x-hasura-role": IPermissions.READINESS_CHECK}},
    })
    expect(screen.getByText("readiness.success")).toBeVisible()
    const rows = screen.getByRole("list", {name: "readiness.results"})
    expect(rows.children).toHaveLength(4)
    expect(screen.getByText("logsScreen.readiness.checks.tested_finished")).toBeVisible()
    expect(screen.queryByText("logsScreen.readiness.checks.publication")).not.toBeInTheDocument()
    expect(screen.getByText(/2026-10-16T09:00:00Z/)).toBeVisible()
    expect(mockRefresh).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole("button", {name: "readiness.download"}))
    await waitFor(() => expect(mockDownload).toHaveBeenCalledTimes(1))
    const [blob, name] = mockDownload.mock.calls[0]
    expect(blob.type).toBe("application/json")
    expect(name).toBe(`readiness-${official}.json`)
    const contents = await new Promise<string>((resolve) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result))
        reader.readAsText(blob)
    })
    expect(JSON.parse(contents)).toEqual(report())
})
it("does not render an old passing official-publication check as a live row", async () => {
    const oldCheck = {kind: "publication", passed: true, message: "Legacy official publication"}
    const old = report()
    mockRun.mockResolvedValue({
        data: {readiness_check: {...old, checks: [...old.checks, oldCheck]}},
    })
    open()
    select()
    await run()
    expect(screen.getByText("readiness.success")).toBeVisible()
    expect(screen.queryByText(oldCheck.message)).not.toBeInTheDocument()
})
it("shows failed checks and retains their evidence for download", async () => {
    mockRun.mockResolvedValue({data: {readiness_check: report(false)}})
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("readiness.failed")).toBeVisible()
    expect(screen.getByText("logsScreen.readiness.checks.certified_voters")).toBeVisible()
    expect(screen.getByRole("button", {name: "readiness.download"})).toBeEnabled()
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(mockRefresh).not.toHaveBeenCalled()
})
it("explains each failed check beside the server's own condition", async () => {
    const failed = report(false)
    failed.checks = failed.checks.map((check) =>
        check.kind === "certified_voters"
            ? {
                  ...check,
                  findings: [
                      {
                          code: "registry_count_changed",
                          event: "official",
                          area: "Area A",
                          values: {imported: "3", current: "4"},
                          message: "The official event's registry has 4 voters in area Area A",
                      },
                      {code: "future_check", event: "tested", message: "A newer server detail"},
                  ],
              }
            : check.kind === "official_zero"
              ? {...check, passed: false}
              : check
    )
    failed.checks.forEach((check, index) => (check.message = `Server condition ${index}`))
    mockRun.mockResolvedValue({data: {readiness_check: failed}})
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("readiness.failed")).toBeVisible()
    for (const check of failed.checks) expect(screen.getByText(check.message)).toBeVisible()
    expect(screen.getByText("readiness.findings.registry_count_changed")).toBeVisible()
    expect(screen.getByText("A newer server detail")).toBeVisible()
    // An older server's failure has no findings: say so rather than invent one.
    expect(screen.getByText("readiness.noDetail")).toBeVisible()
})
it("shows the server's message when the check could not be completed", async () => {
    mockRun.mockResolvedValueOnce({
        data: null,
        errors: [
            {
                message:
                    "Readiness check could not be completed: the official event's evidence could not be read",
            },
        ],
    })
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("readiness.error")).toBeVisible()
    expect(
        screen.getByText(
            "Readiness check could not be completed: the official event's evidence could not be read"
        )
    ).toBeVisible()
    mockRun.mockRejectedValueOnce(
        Object.assign(new Error("Response not successful"), {
            graphQLErrors: [{message: "The official and tested events must differ"}],
        })
    )
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("The official and tested events must differ")).toBeVisible()
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
})
it.each([
    ["transport failure", () => Promise.reject(new Error("Network unavailable"))],
    [
        "GraphQL failure with partial data",
        () => Promise.resolve({data: {readiness_check: report()}, errors: [{message: "denied"}]}),
    ],
    ["missing report", () => Promise.resolve({data: {readiness_check: null}})],
    [
        "unsigned pass",
        () => Promise.resolve({data: {readiness_check: {...report(), log_entry_id: null}}}),
    ],
    [
        "inconsistent verdict",
        () =>
            Promise.resolve({
                data: {readiness_check: {...report(false), passed: true, log_entry_id: "entry"}},
            }),
    ],
])("never displays success for %s and allows an explicit retry", async (_name, answer) => {
    mockRun.mockImplementationOnce(answer)
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("readiness.error")).toBeVisible()
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "readiness.download"})).not.toBeInTheDocument()
    await run()
})
it("disables duplicate submissions and changing the comparison while pending", async () => {
    let finish!: (value: unknown) => void
    mockRun.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                finish = resolve
            })
    )
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(screen.getByRole("combobox", {name: "readiness.testedEvent"})).toBeDisabled()
    expect(comparisonControl()).toHaveAttribute("aria-disabled", "true")
    expect(screen.getByRole("button", {name: "readiness.running"})).toBeDisabled()
    fireEvent.click(screen.getByRole("button", {name: "readiness.running"}))
    expect(mockRun).toHaveBeenCalledTimes(1)
    await act(async () => {
        finish({data: {readiness_check: report()}})
    })
    expect(await screen.findByText("signed-entry-1")).toBeVisible()
})
it("clears the previous report when a different rehearsal is selected", async () => {
    open()
    select()
    await run()
    select("Other rehearsal")
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(screen.queryByText("signed-entry-1")).not.toBeInTheDocument()
})
it("does not permit running against an unavailable event list", async () => {
    mockQuery.mockReturnValue({loading: false, error: new Error("denied"), refetch: jest.fn()})
    open()
    expect(screen.getByText("readiness.eventsError")).toBeVisible()
    expect(screen.getByRole("button", {name: "readiness.run"})).toBeDisabled()
})

it.each([
    ["en", ["Alpha 2", "alpha 10", "Ängel", "Zulu"]],
    ["sv", ["Alpha 2", "alpha 10", "Zulu", "Ängel"]],
])(
    "sorts displayed event names in %s with natural numbering, including archived events",
    (language, names) => {
        mockLanguage = language
        const events = [
            event(other, "Zulu"),
            event(tested, "alpha 10", true),
            event("fifth", "Ängel"),
            event("sixth", "Alpha 2"),
        ]
        mockQuery.mockReturnValue({data: {sequent_backend_election_event: events}, loading: false})
        open()
        fireEvent.mouseDown(screen.getByRole("combobox", {name: "readiness.testedEvent"}))
        expect(
            screen.getAllByRole("option").map((option) => option.textContent?.split(" · ")[0])
        ).toEqual(names)
        expect(events.map((event) => event.id)).toEqual([other, tested, "fifth", "sixth"])
    }
)

const rehearsalRoles = {
    rehearsal_ert_stl: ["ert", "stl"],
    rehearsal_official_ert: ["official", "ert"],
    rehearsal_official_stl: ["official", "stl"],
} as const
const rehearsal = (comparison: keyof typeof rehearsalRoles) => {
    const subset = comparison === "rehearsal_official_stl"
    return {
        comparison,
        verdict: subset ? "not_verified" : "consistent",
        checked_at: "2026-10-16T09:00:00Z",
        checks: [
            "configuration",
            "own_census",
            ...(subset ? ["census_subset"] : []),
            "lifecycle",
            "seal",
        ].map((kind) =>
            kind === "census_subset"
                ? {
                      kind,
                      verdict: "not_verified",
                      message: "Server condition",
                      findings: [
                          {
                              code: "subset_not_verified",
                              event: "both",
                              values: {missing: "stable_voter_identity"},
                              message: "Subset not verified",
                          },
                      ],
                  }
                : {kind, verdict: "consistent", message: "Server condition"}
        ),
        reference: {role: rehearsalRoles[comparison][0], evidence: {event_id: official}},
        compared: {role: rehearsalRoles[comparison][1], evidence: {event_id: tested}},
    }
}
function chooseComparison(mode: string) {
    fireEvent.mouseDown(comparisonControl())
    fireEvent.click(screen.getByRole("option", {name: `readiness.rehearsal.modes.${mode}`}))
}

it.each(Object.keys(rehearsalRoles) as Array<keyof typeof rehearsalRoles>)(
    "runs %s read-only with its discriminant and never shows a signed pass",
    async (mode) => {
        mockRun.mockResolvedValue({data: {readiness_check: rehearsal(mode)}})
        open()
        chooseComparison(mode)
        expect(screen.getByText("readiness.rehearsal.readOnly")).toBeVisible()
        const [current, selected] = rehearsalRoles[mode]
        expect(screen.getByText(`readiness.rehearsal.roles.${current}`)).toBeVisible()
        select("Rehearsal", `readiness.rehearsal.roles.${selected}`)
        fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
        const verdict = mode === "rehearsal_official_stl" ? "not_verified" : "consistent"
        expect(await screen.findByText(`readiness.rehearsal.verdicts.${verdict}`)).toBeVisible()
        expect(mockRun).toHaveBeenCalledWith({
            variables: {official_event_id: official, tested_event_id: tested, comparison: mode},
            context: {headers: {"x-hasura-role": IPermissions.READINESS_CHECK}},
        })
        expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
        expect(
            screen.queryByText("logsScreen.readiness.fields.log_entry_id")
        ).not.toBeInTheDocument()
        expect(mockRefresh).not.toHaveBeenCalled()
        if (mode === "rehearsal_official_stl") {
            expect(screen.getByText("readiness.rehearsal.checks.census_subset")).toBeVisible()
            expect(screen.getByText("readiness.rehearsal.chips.not_verified")).toBeVisible()
        }
        fireEvent.click(screen.getByRole("button", {name: "readiness.download"}))
        await waitFor(() => expect(mockDownload).toHaveBeenCalledTimes(1))
        expect(mockDownload.mock.calls[0][1]).toBe(`readiness-${mode}-${official}.json`)
    }
)
it("refuses a rehearsal answer that claims signed readiness or another mode", async () => {
    for (const forged of [
        {...rehearsal("rehearsal_ert_stl"), passed: true, log_entry_id: "signed-entry-1"},
        rehearsal("rehearsal_official_ert"),
        report(),
    ]) {
        mockRun.mockResolvedValueOnce({data: {readiness_check: forged}})
        open()
        chooseComparison("rehearsal_ert_stl")
        select("Rehearsal", "readiness.rehearsal.roles.stl")
        fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
        expect(await screen.findByText("readiness.error")).toBeVisible()
        expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
        expect(screen.queryByText(/readiness\.rehearsal\.verdicts/)).not.toBeInTheDocument()
        cleanup()
    }
})
it("changing the comparison clears a previous signed report", async () => {
    open()
    select()
    await run()
    chooseComparison("rehearsal_official_ert")
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(screen.queryByText("signed-entry-1")).not.toBeInTheDocument()
})
