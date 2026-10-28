/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, within} from "@testing-library/react"
import {useMutation, useQuery} from "@apollo/client"
import * as verify from "@vitrine/verify"
import {EditElectionEventTransparency} from "./EditElectionEventTransparency"
import type {ITransparencyOutput} from "@/queries/Transparency"
import {draftFileName} from "@/services/Transparency"
import {
    draftPolicyText,
    EVENT_ID,
    OTHER_EVENT_ID,
    policyHexOf,
    policyText,
    sidecarLogs,
    sidecarStatus,
} from "@/fixtures/vitrinePolicy"

jest.mock("@apollo/client", () => ({
    gql: (strings: TemplateStringsArray) => strings.join(""),
    useQuery: jest.fn(),
    useMutation: jest.fn(),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

const ROLE = {headers: {"x-hasura-role": "election-event-transparency-tab"}}
const MARKUP = "<img src=x onerror=alert(1)>"

const enable = jest.fn()
const upload = jest.fn()
const refetch = jest.fn()
const writeText = jest.fn()
const createObjectURL = jest.fn()

const output = (o: Partial<ITransparencyOutput> = {}): ITransparencyOutput => ({
    mode: "disabled",
    refusal: null,
    locked: false,
    logs: [],
    status: {},
    draft_policy: null,
    policy: null,
    policy_id: null,
    enrolments: [],
    explorer_url: null,
    ...o,
})

const publicDraft = (o: Partial<ITransparencyOutput> = {}) =>
    output({mode: "public", draft_policy: draftPolicyText(), ...o})

const answer = (value: ITransparencyOutput | undefined, error?: unknown) => {
    ;(useQuery as jest.Mock).mockReturnValue({
        data: value ? {get_transparency: value} : undefined,
        loading: false,
        error,
        refetch,
    })
}

const renderTab = async (props: {electionEventId?: string; onChanged?: () => void} = {}) => {
    const view = render(<EditElectionEventTransparency electionEventId={EVENT_ID} {...props} />)
    await act(async () => {})
    return view
}

const paste = async (text: string) => {
    await act(async () => {
        fireEvent.change(screen.getByTestId("policy-text"), {target: {value: text}})
    })
}

const draftWords = (text: string) =>
    verify.policyWords(verify.parsePolicy(text, {allowDraft: true, requireSignatures: false}))

beforeAll(async () => {
    await verify.init()
    Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})
    Object.assign(URL, {createObjectURL, revokeObjectURL: jest.fn()})
})

beforeEach(() => {
    jest.clearAllMocks()
    ;(useMutation as jest.Mock).mockImplementation((document: string) => [
        document.includes("EnableTransparency") ? enable : upload,
        {loading: false},
    ])
    writeText.mockResolvedValue(undefined)
    createObjectURL.mockReturnValue("blob:draft")
    jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined)
})

afterEach(() => {
    jest.restoreAllMocks()
})

describe("the event's mode", () => {
    it("asks Harvest as the tab's role, and enables the mode the admin confirms", async () => {
        const changed = jest.fn()
        answer(output())
        enable.mockResolvedValue({
            data: {
                enable_transparency: output({
                    mode: "private",
                    draft_policy: draftPolicyText({visibility: "private"}),
                }),
            },
        })
        await renderTab({onChanged: changed})

        expect(useQuery).toHaveBeenCalledWith(
            expect.stringContaining("get_transparency"),
            expect.objectContaining({
                variables: {electionEventId: EVENT_ID},
                skip: false,
                fetchPolicy: "network-only",
                context: ROLE,
            })
        )
        expect(useMutation).toHaveBeenCalledWith(expect.stringContaining("enable_transparency"), {
            context: ROLE,
        })
        expect(useMutation).toHaveBeenCalledWith(
            expect.stringContaining("upload_transparency_policy"),
            {context: ROLE}
        )
        expect(screen.getByTestId("transparency-mode").textContent).toBe(
            "transparencyTab.mode.disabled"
        )

        const choices = screen.getByTestId("enable-transparency")
        fireEvent.click(within(choices).getByDisplayValue("private"))
        fireEvent.click(screen.getByTestId("enable-transparency-button"))
        expect(enable).not.toHaveBeenCalled()
        await act(async () => {
            fireEvent.click(screen.getByTestId("confirm-enable-transparency"))
        })

        expect(enable).toHaveBeenCalledWith({variables: {electionEventId: EVENT_ID, mode: "private"}})
        expect(screen.getByTestId("transparency-notice").textContent).toContain(
            "transparencyTab.mode.enabled"
        )
        expect(screen.getByTestId("transparency-mode").textContent).toBe(
            "transparencyTab.mode.private"
        )
        expect(screen.getByTestId("draft-words").textContent).toBe(
            draftWords(draftPolicyText({visibility: "private"})).join(" ")
        )
        expect(changed).toHaveBeenCalledTimes(1)
    })

    it("says why public is refused, and offers private instead", async () => {
        answer(output({refusal: "public-needs-unlimited-revotes"}))
        await renderTab()

        const refusal = screen.getByTestId("public-refusal").textContent
        expect(refusal).toContain("transparencyTab.mode.refusals.public-needs-unlimited-revotes")
        expect(refusal).toContain("transparencyTab.mode.refusalOptions")
        const choices = screen.getByTestId("enable-transparency")
        expect((within(choices).getByDisplayValue("public") as HTMLInputElement).disabled).toBe(
            true
        )
        expect((within(choices).getByDisplayValue("private") as HTMLInputElement).checked).toBe(
            true
        )
    })

    it("keeps a refusal Harvest asked for public with, without calling it enabled", async () => {
        const changed = jest.fn()
        answer(output())
        enable.mockResolvedValue({
            data: {enable_transparency: output({refusal: "public-needs-unlimited-revotes"})},
        })
        await renderTab({onChanged: changed})

        fireEvent.click(screen.getByTestId("enable-transparency-button"))
        await act(async () => {
            fireEvent.click(screen.getByTestId("confirm-enable-transparency"))
        })

        expect(enable).toHaveBeenCalledWith({variables: {electionEventId: EVENT_ID, mode: "public"}})
        expect(screen.getByTestId("public-refusal")).toBeTruthy()
        expect(screen.queryByTestId("transparency-notice")).toBeNull()
        expect(changed).not.toHaveBeenCalled()
    })

    it("shows a refusal it doesn't know as text", async () => {
        answer(output({refusal: MARKUP}))
        const {container} = await renderTab()

        expect(screen.getByTestId("public-refusal").textContent).toContain(
            `transparencyTab.mode.refusalUnknown ${JSON.stringify({code: MARKUP})}`
        )
        expect(container.querySelector("img")).toBeNull()
    })

    it("says why enabling failed", async () => {
        answer(output())
        enable.mockRejectedValue({graphQLErrors: [{message: "The event has no sidecar"}]})
        await renderTab()

        fireEvent.click(screen.getByTestId("enable-transparency-button"))
        await act(async () => {
            fireEvent.click(screen.getByTestId("confirm-enable-transparency"))
        })

        expect(screen.getByTestId("transparency-notice").textContent).toBe(
            `transparencyTab.mode.enableError ${JSON.stringify({reason: "The event has no sidecar"})}`
        )
        expect(screen.getByTestId("transparency-mode").textContent).toBe(
            "transparencyTab.mode.disabled"
        )
    })

    it("changes nothing once voting opened", async () => {
        answer(output({mode: "public", locked: true, policy: policyText()}))
        await renderTab()

        expect(screen.getByTestId("transparency-locked")).toBeTruthy()
        expect(screen.queryByTestId("enable-transparency")).toBeNull()
        expect(screen.queryByTestId("upload-policy")).toBeNull()
        expect(screen.getByTestId("sealed-policy")).toBeTruthy()
    })

    it("offers no mode to enable once the event is locked", async () => {
        answer(output({locked: true}))
        await renderTab()

        expect(screen.queryByTestId("enable-transparency")).toBeNull()
        expect(screen.queryByTestId("upload-policy")).toBeNull()
    })
})

describe("the trust policy", () => {
    it("offers the draft to download for the Board, with its eight words", async () => {
        answer(publicDraft())
        await renderTab()

        const words = draftWords(draftPolicyText())
        expect(words).toHaveLength(8)
        expect(screen.getByTestId("draft-words").textContent).toBe(words.join(" "))
        expect(screen.getByTestId("board-commands").textContent).toContain(
            `vitrine policy words ${draftFileName(EVENT_ID)}`
        )

        const click = jest.spyOn(HTMLAnchorElement.prototype, "click")
        fireEvent.click(screen.getByTestId("download-draft"))
        const blob = createObjectURL.mock.calls[0][0] as Blob
        expect(await blob.text()).toBe(draftPolicyText())
        const link = click.mock.instances[0] as unknown as HTMLAnchorElement
        expect(link.download).toBe(draftFileName(EVENT_ID))
        expect(link.href).toBe("blob:draft")
    })

    it("says when Harvest hasn't drafted the policy yet", async () => {
        answer(output({mode: "public"}))
        await renderTab()

        expect(screen.getByTestId("no-draft")).toBeTruthy()
        expect(screen.queryByTestId("draft-policy")).toBeNull()
    })

    it("checks the signed policy before uploading it, then shows its ID and six words", async () => {
        const changed = jest.fn()
        const signed = policyText()
        const summary = verify.policySummary(verify.parsePolicy(signed))
        answer(publicDraft())
        upload.mockResolvedValue({
            data: {
                upload_transparency_policy: publicDraft({
                    policy: signed,
                    policy_id: policyHexOf(signed),
                }),
            },
        })
        await renderTab({onChanged: changed})

        await paste(signed.replace(/\n/g, "\r\n").trimEnd())
        const preview = screen.getByTestId("policy-preview")
        expect(within(preview).getByTestId("candidate-policy-id").textContent).toBe(summary.id)
        expect(within(preview).getByTestId("candidate-policy-words").textContent).toBe(
            summary.words.join(" ")
        )
        expect(screen.queryByTestId("policy-not-from-draft")).toBeNull()

        await act(async () => {
            fireEvent.click(screen.getByTestId("upload-policy-button"))
        })

        expect(upload).toHaveBeenCalledWith({
            variables: {electionEventId: EVENT_ID, signedPolicy: signed},
        })
        expect(screen.getByTestId("transparency-notice").textContent).toBe(
            "transparencyTab.upload.uploaded"
        )
        expect(screen.getByTestId("event-policy-id").textContent).toBe(summary.id)
        expect(screen.getByTestId("event-policy-words").textContent).toBe(summary.words.join(" "))
        expect(screen.queryByTestId("policy-preview")).toBeNull()
        expect(screen.queryByTestId("draft-policy")).toBeNull()
        expect(changed).toHaveBeenCalledTimes(1)
    })

    it("reads the signed policy from a file", async () => {
        answer(publicDraft())
        await renderTab()

        const file = new File([policyText()], "signed.txt", {type: "text/plain"})
        await act(async () => {
            fireEvent.change(screen.getByTestId("policy-file"), {target: {files: [file]}})
        })

        expect((await screen.findByTestId("candidate-policy-id")).textContent).toBe(
            verify.parsePolicy(policyText()).id
        )
    })

    it.each([
        ["an unsealed draft", draftPolicyText(), "policy-is-draft"],
        ["text that isn't a policy", MARKUP, "policy-invalid"],
        ["another event's policy", policyText({eventId: OTHER_EVENT_ID}), "policy-other-event"],
        [
            "a private policy for a public event",
            policyText({visibility: "private"}),
            "policy-other-visibility",
        ],
    ])("refuses to upload %s", async (_what, text, problem) => {
        answer(publicDraft())
        const {container} = await renderTab()

        await paste(text)

        expect(screen.getByTestId(problem)).toBeTruthy()
        const button = screen.queryByTestId("upload-policy-button") as HTMLButtonElement | null
        if (button) {
            expect(button.disabled).toBe(true)
            fireEvent.click(button)
        }
        expect(upload).not.toHaveBeenCalled()
        expect(container.querySelector("img")).toBeNull()
    })

    it("names the draft's words when a draft is pasted", async () => {
        answer(publicDraft())
        await renderTab()

        await paste(draftPolicyText())

        expect(screen.getByTestId("policy-is-draft").textContent).toBe(
            `transparencyTab.upload.draft ${JSON.stringify({
                words: draftWords(draftPolicyText()).join(" "),
            })}`
        )
    })

    it("warns about a policy sealed from another draft of the event", async () => {
        answer(publicDraft({draft_policy: draftPolicyText({name: "acme-2027"})}))
        await renderTab()

        await paste(policyText())

        expect(screen.getByTestId("policy-not-from-draft")).toBeTruthy()
        expect((screen.getByTestId("upload-policy-button") as HTMLButtonElement).disabled).toBe(
            false
        )
    })

    it("says why an upload failed and keeps the pasted policy", async () => {
        answer(publicDraft())
        upload.mockRejectedValue({message: "TransparencyRefused: the logs are frozen\nstack"})
        await renderTab()

        await paste(policyText())
        await act(async () => {
            fireEvent.click(screen.getByTestId("upload-policy-button"))
        })

        expect(screen.getByTestId("transparency-notice").textContent).toBe(
            `transparencyTab.upload.error ${JSON.stringify({
                reason: "TransparencyRefused: the logs are frozen",
            })}`
        )
        expect(screen.getByTestId("policy-preview")).toBeTruthy()
    })

    it("shows the event's policy and offers to upload a successor", async () => {
        const signed = policyText()
        const summary = verify.policySummary(verify.parsePolicy(signed))
        answer(publicDraft({policy: signed, policy_id: policyHexOf(signed)}))
        await renderTab()

        expect(screen.getByTestId("event-policy-id").textContent).toBe(summary.id)
        expect(screen.getByTestId("event-policy-words").textContent).toBe(summary.words.join(" "))
        expect(screen.getByTestId("event-policy").textContent).toContain("acme-2026")
        expect(screen.queryByTestId("policy-id-mismatch")).toBeNull()
        expect(screen.queryByTestId("draft-policy")).toBeNull()
        expect(screen.getByTestId("upload-policy").textContent).toContain(
            "transparencyTab.upload.successorTitle"
        )
    })

    it("warns when the logs report another policy ID", async () => {
        const other = "ab".repeat(32)
        answer(publicDraft({policy: policyText(), policy_id: other}))
        await renderTab()

        expect(screen.getByTestId("policy-id-mismatch").textContent).toContain(other)
    })

    it("offers the new draft when the event changed after the policy was sealed", async () => {
        answer(
            publicDraft({
                policy: policyText(),
                policy_id: policyHexOf(policyText()),
                draft_policy: draftPolicyText({name: "acme-2027"}),
            })
        )
        await renderTab()

        expect(screen.getByTestId("sealed-policy")).toBeTruthy()
        expect(screen.getByTestId("draft-policy").textContent).toContain(
            "transparencyTab.policy.changedDraft"
        )
        expect(screen.getByTestId("draft-words").textContent).toBe(
            draftWords(draftPolicyText({name: "acme-2027"})).join(" ")
        )
    })

    it("says why a stored policy isn't valid", async () => {
        answer(publicDraft({policy: policyText().replace("mmd 60", "mmd 61")}))
        await renderTab()

        expect(screen.getByTestId("stored-policy-invalid")).toBeTruthy()
        expect(screen.queryByTestId("sealed-policy")).toBeNull()
    })
})

describe("the logs", () => {
    const signed = policyText()
    const hex = policyHexOf(signed)
    const withLogs = (o: Partial<ITransparencyOutput> = {}) =>
        publicDraft({
            policy: signed,
            policy_id: hex,
            logs: sidecarLogs(hex),
            status: sidecarStatus(hex),
            ...o,
        })

    it("shows their checkpoints, the witnesses' lag and the hubs' last anchors", async () => {
        answer(withLogs())
        await renderTab()

        const ballots = screen.getByTestId("log-ballots")
        expect(ballots.textContent).toContain("logs.acme.example/e/ballots")
        expect(ballots.textContent).toContain("transparencyTab.logs.policyCurrent")
        expect(screen.getByTestId("witness-w1-lag").textContent).toBe("0")
        expect(screen.getByTestId("witness-w2-lag").textContent).toBe("3")
        expect(screen.getByTestId("witness-w3-lag").textContent).toBe(
            "transparencyTab.logs.unknown"
        )
        expect(screen.getByTestId("witness-w2").textContent).toContain("connection refused")
        expect(screen.getByTestId("witness-w2").textContent).toContain(
            "transparencyTab.logs.witnessAlert"
        )
        expect(screen.getByTestId("hub-hub-a.example").textContent).toContain("8")

        const events = screen.getByTestId("log-events")
        expect(events.textContent).toContain("transparencyTab.logs.noCheckpoint")
        expect(events.textContent).toContain("transparencyTab.logs.noWitnesses")
        expect(screen.getByTestId("log-events-alert").textContent).toContain("witness-quorum-late")
    })

    it("says which logs run under another policy", async () => {
        answer(withLogs({status: sidecarStatus("ef".repeat(32)), logs: []}))
        await renderTab()

        expect(screen.getByTestId("log-ballots").textContent).toContain(
            "transparencyTab.logs.policyOther"
        )
    })

    it("says when the logs haven't reported yet", async () => {
        answer(publicDraft())
        await renderTab()

        expect(screen.getByTestId("no-logs")).toBeTruthy()
    })

    it("links to the explorer over https only", async () => {
        answer(withLogs({explorer_url: "https://explorer.acme.example/e/"}))
        const view = await renderTab()

        const link = screen.getByTestId("explorer-link") as HTMLAnchorElement
        expect(link.href).toBe("https://explorer.acme.example/e/")
        expect(link.target).toBe("_blank")
        expect(link.rel).toBe("noopener noreferrer")

        view.unmount()
        answer(withLogs({explorer_url: "javascript:alert(1)"}))
        await renderTab()
        expect(screen.queryByTestId("explorer-link")).toBeNull()
    })

    it("shows the sidecar's text as text", async () => {
        const status = sidecarStatus(hex) as {ballots: {origin: string}}
        status.ballots.origin = MARKUP
        answer(withLogs({status}))
        const {container} = await renderTab()

        expect(screen.getByTestId("log-ballots").textContent).toContain(MARKUP)
        expect(container.querySelector("img")).toBeNull()
    })
})

describe("witness enrolment", () => {
    it("gives each log's note to copy for the witnesses' operators", async () => {
        const note = `vitrine/enrol/v1\nlogs.acme.example/e/ballots\n${MARKUP}\n`
        answer(publicDraft({enrolments: [{log: "ballots", note}]}))
        const {container} = await renderTab()

        const enrolment = screen.getByTestId("enrolment-ballots")
        expect(enrolment.querySelector("pre")?.textContent).toBe(note)
        expect(container.querySelector("img")).toBeNull()

        await act(async () => {
            fireEvent.click(screen.getByTestId("copy-enrolment-ballots"))
        })
        expect(writeText).toHaveBeenCalledWith(note)
        expect(screen.getByTestId("copy-enrolment-ballots").textContent).toBe(
            "transparencyTab.copied"
        )
    })

    it("says when the note couldn't be copied", async () => {
        writeText.mockRejectedValue(new Error("denied"))
        answer(publicDraft({enrolments: [{log: "ballots", note: "note"}]}))
        await renderTab()

        await act(async () => {
            fireEvent.click(screen.getByTestId("copy-enrolment-ballots"))
        })
        expect(screen.getByTestId("copy-enrolment-ballots").textContent).toBe(
            "transparencyTab.copyFailed"
        )
    })
})

describe("loading", () => {
    it("says why the tab couldn't load, and tries again", async () => {
        answer(undefined, {graphQLErrors: [{message: "Vitrine is unavailable"}]})
        await renderTab()

        expect(screen.getByTestId("transparency-load-error").textContent).toContain(
            JSON.stringify({reason: "Vitrine is unavailable"})
        )
        await act(async () => {
            fireEvent.click(screen.getByText("transparencyTab.retry"))
        })
        expect(refetch).toHaveBeenCalledTimes(1)
    })

    it("asks for nothing without an event", async () => {
        answer(undefined)
        const {container} = await renderTab({electionEventId: undefined})

        expect(container.textContent).toBe("")
        expect(useQuery).toHaveBeenCalledWith(
            expect.anything(),
            expect.objectContaining({skip: true})
        )
    })
})
