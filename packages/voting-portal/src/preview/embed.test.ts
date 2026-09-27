// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ScenarioChannel,
    ScenarioId,
    scenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {DEMO_PUBLIC_KEY, IDS} from "@sequentech/ui-test-kit/fixtures"
import {
    EMBED_PROTOCOL,
    EMBED_VERSION,
    EmbedMessageError,
    EmbedMessageType,
    NOT_A_KEY,
    embedMessage,
    embeddedSource,
    fillLabel,
    keypadHint,
    readCallMessage,
    readShowMessage,
} from "./embed"
import {fakeCallConfig} from "./fakeIvrEmulator"
import {PreviewScreen} from "./screens"

const show = (fields: Record<string, unknown> = {}) => {
    const {preview, areaId} = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    return {
        protocol: EMBED_PROTOCOL,
        version: EMBED_VERSION,
        type: EmbedMessageType.SHOW,
        document: preview,
        areaId,
        screen: PreviewScreen.VOTE,
        ...fields,
    }
}

const issuesOf = (data: unknown) => {
    try {
        readShowMessage(data)
    } catch (error) {
        if (error instanceof EmbedMessageError) return error.issues
        throw error
    }
    throw new Error("The message was accepted")
}

test("messages of other protocols are not for the embed", () => {
    for (const other of [
        undefined,
        "text",
        {type: "webpackOk"},
        {protocol: "another", type: EmbedMessageType.SHOW},
    ])
        expect(readShowMessage(other)).toBeUndefined()
    // Only a parent's request is read; the embed's own replies are not requests.
    expect(readShowMessage(embedMessage({type: EmbedMessageType.READY}))).toBeUndefined()
})

test("a show message names the document, the voter and the screen", () => {
    const request = readShowMessage(show({electionId: IDS.election, language: "es"}))
    expect(request).toMatchObject({
        areaId: IDS.area,
        electionId: IDS.election,
        screen: PreviewScreen.VOTE,
        language: "es",
    })
    expect(request?.document.election_event.id).toBe(IDS.event)
})

test("a message of another version is refused rather than guessed at", () => {
    expect(issuesOf(show({version: EMBED_VERSION + 1}))).toEqual([
        `version: expected ${EMBED_VERSION}, found ${EMBED_VERSION + 1}`,
    ])
})

test("every problem of a show message is reported together", () => {
    const {document} = show()
    const issues = issuesOf(
        show({
            areaId: "",
            screen: "audit",
            electionId: 3,
            language: "",
            channel: "telephone",
            document: {...document, election_event: undefined},
        })
    )
    expect(issues).toEqual([
        'areaId: expected a non-empty string, found ""',
        'screen: expected one of chooser, start, vote, review, confirmation, found "audit"',
        "electionId: expected a non-empty string, found 3",
        'language: expected a non-empty string, found ""',
        'channel: expected one of online, kiosk, found "telephone"',
        "preview.election_event: expected an object, found nothing",
    ])
})

test("the source uses the event's tenant and an online voter unless told otherwise", () => {
    const request = readShowMessage(show())!
    const source = embeddedSource(request)
    expect(source).toMatchObject({
        tenantId: IDS.tenant,
        areaId: IDS.area,
        channel: ScenarioChannel.ONLINE,
    })
    expect(source.preview).toBe(request.document)

    const kiosk = readShowMessage(show({channel: ScenarioChannel.KIOSK, electionId: "e"}))!
    expect(embeddedSource(kiosk)).toMatchObject({channel: ScenarioChannel.KIOSK, electionId: "e"})

    const {document} = show()
    const withoutTenant = readShowMessage(
        show({document: {...document, election_event: {id: IDS.event}}})
    )!
    expect(embeddedSource(withoutTenant).tenantId).toBe("preview")
})

test("replies carry the protocol and version", () => {
    expect(
        embedMessage({type: EmbedMessageType.SHOWN, screen: PreviewScreen.REVIEW, path: "/p"})
    ).toEqual({
        protocol: EMBED_PROTOCOL,
        version: EMBED_VERSION,
        type: EmbedMessageType.SHOWN,
        screen: PreviewScreen.REVIEW,
        path: "/p",
    })
})

test("a document from before its key ceremony is previewed with the demo key", () => {
    // The Election Architect's core writes a stand-in that is deliberately not a key,
    // so nothing can be encrypted to it. The embed never sends a ballot anywhere, and
    // review and confirmation need one encrypted, so it shows them as a demo.
    const {document} = show()
    const [style] = document.ballot_styles
    const stand_in = {...style, public_key: {public_key: NOT_A_KEY, is_demo: true}}
    const request = readShowMessage(show({document: {...document, ballot_styles: [stand_in]}}))!

    const source = embeddedSource(request)
    expect(source.preview.ballot_styles[0]?.public_key).toEqual({
        public_key: DEMO_PUBLIC_KEY,
        is_demo: true,
    })
    // A real key is left alone, and the request's own document is not changed.
    expect(request.document.ballot_styles[0]?.public_key).toEqual({
        public_key: NOT_A_KEY,
        is_demo: true,
    })
    const plain = readShowMessage(show())!
    expect(embeddedSource(plain).preview).toBe(plain.document)
})

const EMULATOR = "https://architect.example/wasm/ivr_emulator_wasm"

const call = (fields: Record<string, unknown> = {}) => ({
    protocol: EMBED_PROTOCOL,
    version: EMBED_VERSION,
    type: EmbedMessageType.CALL,
    config: fakeCallConfig(),
    emulatorUrl: EMULATOR,
    ...fields,
})

const callIssuesOf = (data: unknown) => {
    try {
        readCallMessage(data)
    } catch (error) {
        if (error instanceof EmbedMessageError) return error.issues
        throw error
    }
    throw new Error("The message was accepted")
}

test("a call names the emulator, what it is given and the words around it", () => {
    const labels = {input: "Teclas", send: "Pulsar"}
    expect(readCallMessage(call({labels}))).toEqual({
        config: fakeCallConfig(),
        emulatorUrl: EMULATOR,
        labels,
    })
    // Each reader reads its own type only.
    expect(readShowMessage(call())).toBeUndefined()
    expect(readCallMessage(show())).toBeUndefined()
    expect(readCallMessage({type: "webpackOk"})).toBeUndefined()
})

test("a call of another version is refused", () => {
    expect(callIssuesOf(call({version: 1}))).toEqual([
        `version: expected ${EMBED_VERSION}, found 1`,
    ])
})

test("every problem of a call is reported together", () => {
    const issues = callIssuesOf(
        call({
            emulatorUrl: "javascript:alert(1)",
            config: {...fakeCallConfig(), contact_id: "", ballot_styles: [], open_elections: [3]},
            labels: {send: "", timeout: 4},
        })
    )
    expect(issues).toEqual([
        'emulatorUrl: expected an absolute http(s) URL, found "javascript:alert(1)"',
        'config.contact_id: expected a non-empty string, found ""',
        "config.open_elections: expected a list of strings, found [3]",
        "config.ballot_styles: a call needs at least one ballot style",
        "labels.timeout: expected a non-empty string, found 4",
        'labels.send: expected a non-empty string, found ""',
    ])
    expect(callIssuesOf(call({emulatorUrl: "/wasm/ivr", config: "x", labels: []}))).toEqual([
        'emulatorUrl: expected an absolute http(s) URL, found "/wasm/ivr"',
        'config: expected an object, found "x"',
        "labels: expected an object, found []",
    ])
})

test("a label's placeholders are filled in, and unknown ones left alone", () => {
    expect(
        fillLabel("Up to {{maxDigits}} of {{ validInputs }} in {{timeout}}s {{other}}", {
            maxDigits: 2,
            validInputs: "0-9",
            timeout: 5,
        })
    ).toBe("Up to 2 of 0-9 in 5s {{other}}")
})

describe("keypadHint", () => {
    const labels = {
        placeholder: "Press {{keys}}, within {{timeout}}s",
        placeholderAnyKeys: "Up to {{maxDigits}} digits, within {{timeout}}s",
        or: "or",
    }

    test("names the keys a prompt accepts, in order, as a sentence", () => {
        // The Election Architect's language menu said "Up to 1 of 2,1, within 5s".
        expect(keypadHint(labels, {valid_inputs: "2,1", max_digits: 1, timeout: 5})).toBe(
            "Press 1 or 2, within 5s"
        )
        expect(keypadHint(labels, {valid_inputs: "1,2,0", max_digits: 1, timeout: 5})).toBe(
            "Press 0, 1 or 2, within 5s"
        )
        expect(keypadHint(labels, {valid_inputs: "1", max_digits: 1, timeout: 10})).toBe(
            "Press 1, within 10s"
        )
    })

    test("puts the symbols after the digits and names each key once", () => {
        expect(keypadHint(labels, {valid_inputs: "#, 2,1,2", max_digits: 1, timeout: 5})).toBe(
            "Press 1, 2 or #, within 5s"
        )
        // The Lambda pads a longer menu's numbers to one length: "01,00,03".
        expect(keypadHint(labels, {valid_inputs: "03,01,10", max_digits: 2, timeout: 5})).toBe(
            "Press 01, 03 or 10, within 5s"
        )
    })

    test("says any digits will do when the Lambda lists no keys", () => {
        // The Lambda's wire format: no valid inputs means any key, up to max_digits (a PIN).
        for (const valid_inputs of ["", "  "]) {
            const hint = keypadHint(labels, {valid_inputs, max_digits: 8, timeout: 5})
            expect(hint).toBe("Up to 8 digits, within 5s")
            expect(hint).not.toMatch(/ of ,|\{\{/)
        }
    })

    test("a framing tool's own words are used for both, its 'or' included", () => {
        const spanish = {
            placeholder: "Pulse {{keys}}, en {{timeout}} s",
            placeholderAnyKeys: "Hasta {{maxDigits}} dígitos en {{timeout}} s",
            or: "o",
        }
        expect(keypadHint(spanish, {valid_inputs: "", max_digits: 4, timeout: 10})).toBe(
            "Hasta 4 dígitos en 10 s"
        )
        expect(keypadHint(spanish, {valid_inputs: "2,1", max_digits: 1, timeout: 10})).toBe(
            "Pulse 1 o 2, en 10 s"
        )
    })

    test("an older framing tool's placeholder still gets the Lambda's own list", () => {
        // EMBED_VERSION 2 documented `{{validInputs}}` as the keys as the Lambda sent
        // them; the sentence is `{{keys}}`, so a parent written against that still reads.
        const older = {...labels, placeholder: "Up to {{maxDigits}} of {{validInputs}}"}
        expect(keypadHint(older, {valid_inputs: "2,1", max_digits: 1, timeout: 5})).toBe(
            "Up to 1 of 2,1"
        )
    })

    test("says 'or' in English when a framing tool sends no word for it", () => {
        const {or: _, ...older} = labels
        expect(keypadHint(older, {valid_inputs: "2,1", max_digits: 1, timeout: 5})).toBe(
            "Press 1 or 2, within 5s"
        )
    })
})

test("a framing tool's word for 'or' has to be a word", () => {
    expect(callIssuesOf(call({labels: {or: ""}}))).toEqual([
        'labels.or: expected a non-empty string, found ""',
    ])
})
