// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMessageAttemptState,
    EMessagePurpose,
    EMessagingProvider,
    EPhoneFormat,
    IAccountSender,
} from "@/types/messaging"
import {
    EHttpConfigProblem,
    EHttpSection,
    HTTP_API_EXAMPLE,
    HTTP_PLACEHOLDERS,
    HTTP_SECTION_SKELETONS,
    emptyHttpApiForm,
    httpFormFromSender,
    httpFormProblems,
    httpSenderFromForm,
    validateHttpJwt,
    validateHttpReconcile,
    validateHttpReports,
    validateHttpRequest,
    validateHttpToken,
    withHttpExample,
} from "./httpApiSender"

const problems = (found: Array<{problem: EHttpConfigProblem; path: string}>) =>
    found.map(({problem, path}) => `${problem} ${path}`.trim())

const STATUS = {
    message_id_pointer: "/id",
    state_pointer: "/status",
    states: {delivered: EMessageAttemptState.DELIVERED},
}

describe("validateHttpRequest", () => {
    it("accepts a request with only a URL", () => {
        expect(validateHttpRequest({url: "https://gateway.example/sms"})).toEqual([])
    })

    it("accepts every documented placeholder in the URL, headers and body", () => {
        expect(
            validateHttpRequest({
                method: "POST",
                url: "https://partner.example/{{message_id}}?cb={{callback_url}}",
                headers: {
                    "Authorization": "Bearer {{credential.API_KEY}}",
                    "X-Basic": "Basic {{basic_auth}}",
                    "X-Token": "{{token}} {{jwt}}",
                },
                body: {
                    to: "{{to}}",
                    text: "{{text}}",
                    subject: "{{subject}}",
                    html: "{{html}}",
                    code: "{{code}}",
                    template: {name: "{{template}}", language: "{{language}}"},
                    first: ["{{param.1}}", "{{param.12}}"],
                    all: "{{parameters}}",
                    named: "{{named_parameters}}",
                },
            })
        ).toEqual([])
    })

    it("needs an object with a URL", () => {
        expect(problems(validateHttpRequest("https://x"))).toEqual(["NOT_AN_OBJECT"])
        expect(problems(validateHttpRequest(null))).toEqual(["NOT_AN_OBJECT"])
        expect(problems(validateHttpRequest({method: "POST"}))).toEqual(["MISSING_URL url"])
        expect(problems(validateHttpRequest({url: " "}))).toEqual(["MISSING_URL url"])
    })

    it("names an invalid method, header or unknown field", () => {
        expect(
            problems(
                validateHttpRequest({
                    url: "https://x",
                    method: 7,
                    headers: {"Accept": "application/json", "X-Retries": 3},
                    payload: {},
                })
            )
        ).toEqual([
            "INVALID_METHOD method",
            "INVALID_HEADERS headers.X-Retries",
            "UNKNOWN_FIELD payload",
        ])
        expect(problems(validateHttpRequest({url: "https://x", headers: []}))).toEqual([
            "INVALID_HEADERS headers",
        ])
    })

    it("names a placeholder that does not exist, wherever it is", () => {
        expect(
            problems(
                validateHttpRequest({
                    url: "https://x/{{recipient}}",
                    headers: {Authorization: "Bearer {{credential.PRIVATE_KEY}}"},
                    body: {items: [{to: "{{ to }}"}], n: "{{param.0}}"},
                })
            )
        ).toEqual([
            "UNKNOWN_PLACEHOLDER url",
            "UNKNOWN_PLACEHOLDER headers.Authorization",
            "UNKNOWN_PLACEHOLDER body.items.0.to",
            "UNKNOWN_PLACEHOLDER body.n",
        ])
    })
})

describe("validateHttpToken", () => {
    it("needs a request and a pointer to the token", () => {
        expect(
            validateHttpToken({
                request: {url: "https://x/oauth/token", body: {client: "{{credential.API_KEY}}"}},
                token_pointer: "/access_token",
                lifetime_seconds: 3600,
            })
        ).toEqual([])
        expect(problems(validateHttpToken({token_pointer: "access_token"}))).toEqual([
            "NOT_AN_OBJECT request",
            "INVALID_POINTER token_pointer",
        ])
        expect(
            problems(
                validateHttpToken({
                    request: {url: "https://x"},
                    token_pointer: "/t",
                    lifetime_seconds: 0,
                })
            )
        ).toEqual(["INVALID_LIFETIME lifetime_seconds"])
    })
})

describe("validateHttpJwt", () => {
    it("accepts claims with placeholders and both algorithms", () => {
        expect(
            validateHttpJwt({
                algorithm: "HS256",
                claims: {iss: "{{credential.API_KEY}}", aud: "partner"},
                lifetime_seconds: 60,
            })
        ).toEqual([])
        expect(validateHttpJwt({})).toEqual([])
    })

    it("names an unknown algorithm, claims that are not an object and a bad lifetime", () => {
        expect(
            problems(validateHttpJwt({algorithm: "ES256", claims: [], lifetime_seconds: "5m"}))
        ).toEqual([
            "INVALID_ALGORITHM algorithm",
            "INVALID_CLAIMS claims",
            "INVALID_LIFETIME lifetime_seconds",
        ])
        expect(problems(validateHttpJwt({claims: {sub: "{{subject_id}}"}}))).toEqual([
            "UNKNOWN_PLACEHOLDER claims.sub",
        ])
    })
})

describe("validateHttpReports", () => {
    it("accepts every way of authenticating callbacks", () => {
        for (const auth of [
            undefined,
            {kind: "URL_KEY"},
            {kind: "HEADER_SECRET", header: "X-Secret"},
            {kind: "HMAC_SHA256", header: "X-Signature", prefix: "sha256=", encoding: "BASE64"},
            {
                kind: "HMAC_SHA256",
                header: "X-Signature",
                signed: "{{body}}.{{header.x-nonce}}.{{header.x-timestamp}}",
            },
            {kind: "JWT_HS256", header: "Authorization"},
        ]) {
            expect(validateHttpReports({auth, status: STATUS})).toEqual([])
        }
    })

    it("names what is wrong with the authentication", () => {
        expect(problems(validateHttpReports({auth: {kind: "BASIC"}, status: STATUS}))).toEqual([
            "INVALID_AUTH auth.kind",
        ])
        expect(
            problems(validateHttpReports({auth: {kind: "HEADER_SECRET"}, status: STATUS}))
        ).toEqual(["INVALID_AUTH auth.header"])
        expect(
            problems(
                validateHttpReports({
                    auth: {
                        kind: "HMAC_SHA256",
                        header: "X-Sig",
                        encoding: "BASE32",
                        signed: "{{to}}",
                    },
                    status: STATUS,
                })
            )
        ).toEqual(["INVALID_AUTH auth.encoding", "UNKNOWN_PLACEHOLDER auth.signed"])
    })

    it("needs a status mapping with pointers and known states", () => {
        expect(problems(validateHttpReports({}))).toEqual(["NOT_AN_OBJECT status"])
        expect(
            problems(
                validateHttpReports({
                    items_pointer: "reports",
                    inbound_from_pointer: "/from",
                    status: {
                        message_id_pointer: "id",
                        state_pointer: "/status",
                        states: {ok: "SENT"},
                        error_pointer: 4,
                    },
                })
            )
        ).toEqual([
            "INVALID_POINTER items_pointer",
            "INVALID_POINTER status.message_id_pointer",
            "INVALID_STATES status.states.ok",
            "INVALID_POINTER status.error_pointer",
        ])
        expect(
            problems(
                validateHttpReports({
                    status: {message_id_pointer: "/id", state_pointer: "/s", states: {}},
                })
            )
        ).toEqual(["INVALID_STATES status.states"])
    })
})

describe("validateHttpReconcile", () => {
    it("needs a request and a status mapping", () => {
        expect(
            validateHttpReconcile({
                request: {method: "GET", url: "https://x/messages/{{message_id}}"},
                status: STATUS,
            })
        ).toEqual([])
        expect(problems(validateHttpReconcile({request: {url: ""}}))).toEqual([
            "MISSING_URL request.url",
            "NOT_AN_OBJECT status",
        ])
    })
})

describe("the reference", () => {
    it("lists every placeholder of the contract once", () => {
        expect(HTTP_PLACEHOLDERS.map(({placeholder}) => placeholder)).toEqual([
            "{{to}}",
            "{{text}}",
            "{{subject}}",
            "{{html}}",
            "{{code}}",
            "{{template}}",
            "{{language}}",
            "{{message_id}}",
            "{{callback_url}}",
            "{{param.1}}",
            "{{credential.NAME}}",
            "{{basic_auth}}",
            "{{token}}",
            "{{jwt}}",
            "{{parameters}}",
            "{{named_parameters}}",
        ])
    })

    it("offers a worked example that is valid", () => {
        const form = withHttpExample(emptyHttpApiForm())
        expect(httpFormProblems(form)).toEqual({})
        expect(form.messageIdPointer).toBe(HTTP_API_EXAMPLE.message_id_pointer)
        expect(JSON.stringify(form.send)).toContain("Bearer {{credential.API_KEY}}")
        expect(JSON.stringify(form.send)).toContain('"{{parameters}}"')
        expect(form.sections[EHttpSection.REPORTS]).toEqual(HTTP_API_EXAMPLE.reports)
    })

    it("starts every optional section from a skeleton of the right shape", () => {
        expect(problems(validateHttpRequest(HTTP_SECTION_SKELETONS.CHECK))).toEqual([
            "MISSING_URL url",
        ])
        expect(problems(validateHttpJwt(HTTP_SECTION_SKELETONS.JWT))).toEqual([])
        expect(problems(validateHttpToken(HTTP_SECTION_SKELETONS.TOKEN))).toEqual([
            "MISSING_URL request.url",
        ])
    })
})

describe("the form", () => {
    it("reports the problems of each section, and a missing send request", () => {
        const form = {
            ...emptyHttpApiForm(),
            messageIdPointer: "id",
            conversationWindowHours: "a day",
            sections: {
                ...emptyHttpApiForm().sections,
                [EHttpSection.CHECK]: {url: "https://x/{{nope}}"},
            },
        }
        const found = httpFormProblems(form)
        expect(Object.keys(found).sort()).toEqual([
            "CHECK",
            "CONVERSATION_WINDOW",
            "MESSAGE_ID_POINTER",
            "SEND",
        ])
        expect(problems(found.SEND ?? [])).toEqual(["MISSING_URL url"])
        expect(problems(found.CHECK ?? [])).toEqual(["UNKNOWN_PLACEHOLDER url"])
    })

    it("builds the sender, leaving out what is not configured", () => {
        const form = {
            ...withHttpExample(emptyHttpApiForm()),
            phoneFormat: EPhoneFormat.DIGITS,
            templateRequiredFor: [EMessagePurpose.OTP],
            conversationWindowHours: " 24 ",
            approvedLanguages: {OTP: "en, tl en", NOTICE: " "},
        }
        expect(httpSenderFromForm(" COMELEC ", form)).toEqual({
            provider: EMessagingProvider.HTTP_API,
            label: "COMELEC",
            send: HTTP_API_EXAMPLE.send,
            message_id_pointer: "/message_id",
            phone_format: EPhoneFormat.DIGITS,
            template_required_for: [EMessagePurpose.OTP],
            conversation_window_hours: 24,
            check: null,
            token: null,
            jwt: null,
            reports: HTTP_API_EXAMPLE.reports,
            reconcile: null,
            approved_templates: {OTP: ["en", "tl"]},
        })
        expect(httpSenderFromForm("", emptyHttpApiForm())).toMatchObject({
            label: null,
            message_id_pointer: null,
            phone_format: EPhoneFormat.E164,
            template_required_for: [],
            conversation_window_hours: null,
            approved_templates: {},
        })
    })

    it("round-trips a stored sender", () => {
        const sender = httpSenderFromForm("COMELEC", {
            ...withHttpExample(emptyHttpApiForm()),
            templateRequiredFor: [EMessagePurpose.OTP, EMessagePurpose.NOTICE],
            approvedLanguages: {OTP: "en", NOTICE: "en, fil"},
        })
        expect(httpSenderFromForm("COMELEC", httpFormFromSender(sender))).toEqual(sender)
    })

    it("reads a sender of another provider as an empty form", () => {
        const sender: IAccountSender = {provider: EMessagingProvider.CONSOLE}
        expect(httpFormFromSender(sender)).toEqual(emptyHttpApiForm())
    })
})
