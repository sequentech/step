/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {readFileSync} from "fs"
import {resolve} from "path"
import {MemoryRouter, useLocation} from "react-router-dom"
import {fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {useQuery} from "@apollo/client"
import {print} from "graphql"
import {EMessageChannel} from "@/types/messaging"
import {EmailInbox} from "./EmailInbox"
import {MessagingTabs} from "./MessagingTabs"
import {AllMessagesTable} from "./SentMessages"
import {emailSrcDoc} from "./emailFrame"
import {FAKE_PROVIDER_REASONS, conversations, demoLabel, demoMessageText} from "./messageContent"
import {FAKE_PROVIDER_MESSAGE, FAKE_PROVIDER_MESSAGES} from "./queries"
import {IFakeProviderMessage, parseMode} from "./types"
import {channelMessages, fullMessage, sentMessages} from "./__stories__/SentMessagesFixture"

// React Router needs the encoders jsdom lacks.
jest.mock("react-router-dom", () => {
    const {TextEncoder, TextDecoder} = jest.requireActual("node:util")
    Object.assign(globalThis, {TextEncoder, TextDecoder})
    return jest.requireActual("react-router-dom")
})
jest.mock("@apollo/client", () => ({
    gql: jest.requireActual("@apollo/client").gql,
    useQuery: jest.fn(),
    useMutation: () => [jest.fn()],
    NetworkStatus: {refetch: 4},
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
let mockCanWrite = true
jest.mock("@/providers/AuthContextProvider", () => {
    const {createContext} = jest.requireActual("react")
    return {AuthContext: createContext({isAuthorized: () => mockCanWrite})}
})
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ChannelIcon: () => null,
        ChannelLabel: ({label}: {label: string}) => <span>{label}</span>,
    }),
    {virtual: true}
)

const useQueryMock = useQuery as jest.Mock

/** The list action answered with the fixture. */
const answered = () =>
    useQueryMock.mockReturnValue({
        data: {messaging_fake_provider_messages: {messages: sentMessages()}},
        previousData: undefined,
        error: undefined,
        networkStatus: 7,
        refetch: jest.fn(() => Promise.resolve()),
    })

/** The list action failed, as it does where the fake provider routes aren't mounted. */
const failed = () =>
    useQueryMock.mockReturnValue({
        data: undefined,
        previousData: undefined,
        error: new Error("http exception when calling webhook"),
        networkStatus: 8,
        refetch: jest.fn(() => Promise.resolve()),
    })

/** The list answers with the fixture; the single-email read with `full`. */
const reads = (full: IFakeProviderMessage | null) =>
    useQueryMock.mockImplementation((document) =>
        document === FAKE_PROVIDER_MESSAGE
            ? {
                  data: full ? {messaging_fake_provider_message: full} : undefined,
                  loading: false,
                  error: undefined,
              }
            : {
                  data: {messaging_fake_provider_messages: {messages: sentMessages()}},
                  previousData: undefined,
                  error: undefined,
                  networkStatus: 7,
                  refetch: jest.fn(() => Promise.resolve()),
              }
    )
const emailReads = () =>
    useQueryMock.mock.calls.filter(([document]) => document === FAKE_PROVIDER_MESSAGE)

const Location: React.FC = () => <output data-testid="search">{useLocation().search}</output>

const renderTabs = (url: string) =>
    render(
        <MemoryRouter initialEntries={[url]}>
            <MessagingTabs accounts={<p>accounts list</p>} refreshMs={0} />
            <Location />
        </MemoryRouter>
    )

beforeEach(() => {
    mockCanWrite = true
    useQueryMock.mockReset()
})

it("keeps a message expanded when a refresh inserts a newer message", () => {
    const original = channelMessages(EMessageChannel.SMS)[0]
    const {rerender} = render(<AllMessagesTable messages={[original]} />)
    fireEvent.click(screen.getByRole("button", {name: "sentMessages.showMore"}))
    expect(screen.getByRole("group", {name: "sentMessages.column.message"})).toHaveTextContent(
        original.content.text!
    )
    rerender(
        <AllMessagesTable
            messages={[
                {
                    ...original,
                    id: "new-message",
                    received_at: "2026-10-11T10:00:00Z",
                    content: {...original.content, text: "A new notice"},
                },
                {...original, report: {...original.report!, status: 200}},
            ]}
        />
    )
    const detail = screen.getByRole("group", {name: "sentMessages.column.message"})
    expect(detail).toHaveTextContent(original.content.text!)
    expect(within(detail).queryByText("A new notice")).toBeNull()
    fireEvent.click(within(detail).getByRole("button", {name: "sentMessages.showLess"}))
    expect(screen.queryByRole("group", {name: "sentMessages.column.message"})).toBeNull()
})

it.each([
    ["Fake WhatsApp", "Demo WhatsApp"],
    ["FakeElections", "DemoElections"],
    ["fake-wa-number-1", "demo-wa-number-1"],
    [
        "Elections <elections@fake-provider.example.org>",
        "Elections <elections@demo-provider.example.org>",
    ],
    [
        "votes@staff-association.fake-provider.example.org",
        "votes@staff-association.demo-provider.example.org",
    ],
    ["Staff Association", "Staff Association"],
    ["real@example.org", "real@example.org"],
])("displays the demo sender label %s as %s", (before, after) => {
    expect(demoLabel(before)).toBe(after)
})

describe("the Sent messages mode in the URL", () => {
    it("opens the tab in the mode `?channel=` names", () => {
        answered()
        renderTabs("/settings?channel=viber")
        expect(screen.getByRole("tab", {name: "sentMessages.tab"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        expect(screen.getByRole("button", {name: "messaging.channel.VIBER"})).toHaveAttribute(
            "aria-pressed",
            "true"
        )
        expect(screen.getByRole("region", {name: "sentMessages.chat.phone"})).toBeInTheDocument()
    })

    it("keeps a mode change in `?channel=`, alongside the page's other parameters", () => {
        answered()
        renderTabs("/settings?tabIndex=2&channel=all")
        expect(screen.getByRole("table", {name: "sentMessages.tab"})).toBeInTheDocument()
        fireEvent.click(screen.getByRole("button", {name: "messaging.channel.EMAIL"}))
        expect(screen.getByTestId("search")).toHaveTextContent("?tabIndex=2&channel=email")
        expect(screen.getByRole("list", {name: "sentMessages.email.inbox"})).toBeInTheDocument()
    })

    it("selects the tab with `?channel=all` and leaves it by removing the parameter", () => {
        answered()
        renderTabs("/settings?tabIndex=2")
        expect(screen.getByText("accounts list")).toBeInTheDocument()
        fireEvent.click(screen.getByRole("tab", {name: "sentMessages.tab"}))
        expect(screen.getByTestId("search")).toHaveTextContent("?tabIndex=2&channel=all")
        fireEvent.click(screen.getByRole("tab", {name: "sentMessages.accountsTab"}))
        expect(screen.getByTestId("search")).toHaveTextContent("?tabIndex=2")
        expect(screen.getByTestId("search")).not.toHaveTextContent("channel")
        expect(screen.getByText("accounts list")).toBeInTheDocument()
    })

    it("reads any other value as All", () => {
        expect(parseMode("WhatsApp")).toBe(EMessageChannel.WHATSAPP)
        expect(parseMode(" sms ")).toBe(EMessageChannel.SMS)
        expect(parseMode("fax")).toBe("ALL")
        expect(parseMode(null)).toBe("ALL")
    })

    it("polls only while the tab is open", () => {
        answered()
        render(
            <MemoryRouter initialEntries={["/settings?channel=sms"]}>
                <MessagingTabs accounts={<p>accounts list</p>} refreshMs={5000} />
            </MemoryRouter>
        )
        expect(useQueryMock.mock.calls.at(-1)?.[1]).toMatchObject({pollInterval: 5000})
        fireEvent.click(screen.getByRole("tab", {name: "sentMessages.accountsTab"}))
        expect(useQueryMock.mock.calls.at(-1)?.[1]).toMatchObject({pollInterval: 0})
    })
})

describe("the Sent messages tab", () => {
    it("shows legacy demo senders without changing the stored message or code", () => {
        const message = {...sentMessages()[0], sender: "FakeElections"}
        useQueryMock.mockReturnValue({
            data: {messaging_fake_provider_messages: {messages: [message]}},
            networkStatus: 7,
            refetch: jest.fn(),
        })
        renderTabs("/settings?channel=all")
        expect(screen.getByText("DemoElections")).toBeVisible()
        expect(screen.queryByText("FakeElections")).toBeNull()
        expect(message.sender).toBe("FakeElections")
        expect(message.content.code).toBe(sentMessages()[0].content.code)
    })

    it("is hidden when the list action fails, even with `?channel=` in the URL", () => {
        failed()
        renderTabs("/settings?channel=whatsapp")
        expect(screen.queryByRole("tablist")).toBeNull()
        expect(screen.queryByRole("tab", {name: "sentMessages.tab"})).toBeNull()
        expect(screen.getByText("accounts list")).toBeInTheDocument()
    })

    it("isn't asked for without messaging-account-write, and stays hidden", () => {
        mockCanWrite = false
        failed()
        renderTabs("/settings?channel=whatsapp")
        expect(useQueryMock.mock.calls.at(-1)?.[1]).toMatchObject({skip: true})
        expect(screen.queryByRole("tablist")).toBeNull()
    })

    it("stays once shown when a refresh fails, with the last list and a warning", () => {
        useQueryMock.mockReturnValue({
            data: undefined,
            previousData: {messaging_fake_provider_messages: {messages: sentMessages()}},
            error: new Error("http exception when calling webhook"),
            networkStatus: 8,
            refetch: jest.fn(() => Promise.resolve()),
        })
        renderTabs("/settings?channel=all")
        expect(screen.getByRole("tab", {name: "sentMessages.tab"})).toBeInTheDocument()
        expect(screen.getByText("sentMessages.refreshError")).toBeInTheDocument()
        expect(screen.getAllByRole("row")).toHaveLength(sentMessages().length + 1)
    })
})

describe("the email reading pane", () => {
    it("shows the HTML in a sandboxed iframe that allows nothing", () => {
        reads(fullMessage("fake-9"))
        render(<EmailInbox messages={channelMessages(EMessageChannel.EMAIL)} />)
        const frame = screen.getByTitle("sentMessages.email.htmlFrame")
        expect(frame.tagName).toBe("IFRAME")
        expect(frame).toHaveAttribute("sandbox", "")
        expect(frame.getAttribute("srcdoc")).toContain("<b>845120</b>")
        expect(frame.getAttribute("srcdoc")).toContain("<script>")
        expect(frame).not.toHaveAttribute("src")
        // The script stays inside the sandboxed frame's document; nothing ran here.
        expect(document.body).not.toHaveTextContent("script ran")
    })

    it("shows a text-only email as text, with no frame", () => {
        reads(fullMessage("fake-9"))
        render(<EmailInbox messages={channelMessages(EMessageChannel.EMAIL)} />)
        fireEvent.click(screen.getByText("Your enrollment was received"))
        expect(screen.queryByTitle("sentMessages.email.htmlFrame")).toBeNull()
        expect(screen.getByText(/We received your enrollment/)).toBeInTheDocument()
    })
})

describe("fix round 2 (R6)", () => {
    const CSP =
        "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; style-src 'unsafe-inline'\">"
    const HOSTILE =
        '<html><head><link rel="stylesheet" href="https://evil.example/x.css">' +
        '<META HTTP-EQUIV="Refresh" content="0;url=https://evil.example">' +
        '<base href="https://evil.example/"></head>' +
        '<body><img src="https://evil.example/p.png"><p>Use <b>845120</b></p>' +
        '<meta http-equiv="refresh" content="5"></body></html>'
    const emails = () => channelMessages(EMessageChannel.EMAIL)
    const withHtml = (message: IFakeProviderMessage, html: string) => ({
        ...message,
        content: {...message.content, html},
    })
    it("R6-05: doesn't poll a deep link while the action has never answered", () => {
        failed()
        render(
            <MemoryRouter initialEntries={["/settings?channel=whatsapp"]}>
                <MessagingTabs accounts={<p>accounts list</p>} refreshMs={5000} />
            </MemoryRouter>
        )
        expect(useQueryMock.mock.calls.at(-1)?.[1]).toMatchObject({pollInterval: 0})
    })

    it("R6-03: the list asks for no HTML; only the selected email's HTML is read", () => {
        expect(print(FAKE_PROVIDER_MESSAGES)).not.toMatch(/\bhtml\b/)
        expect(print(FAKE_PROVIDER_MESSAGES)).toMatch(/\bhas_html\b/)
        expect(print(FAKE_PROVIDER_MESSAGE)).toMatch(/\bhtml\b/)
        const [textOnly, html] = emails()
        reads(fullMessage(html.id))
        render(<EmailInbox messages={emails()} />)
        // The newest email, which has HTML, is open: its HTML alone is read.
        expect(emailReads().at(-1)?.[1]).toMatchObject({variables: {id: html.id}, skip: false})
        expect(
            emailReads().every(([, options]) => options.skip || options.variables.id === html.id)
        ).toBe(true)
        fireEvent.click(screen.getByText("Your enrollment was received"))
        expect(emailReads().at(-1)?.[1]).toMatchObject({skip: true})
        expect(textOnly.content.has_html).toBe(false)
        expect(html.content).not.toHaveProperty("html")
    })

    it("R6-06: the CSP meta comes first in srcDoc, and meta refresh and base are stripped", () => {
        reads(withHtml(fullMessage("fake-9") as IFakeProviderMessage, HOSTILE))
        render(<EmailInbox messages={emails()} />)
        const srcdoc = screen.getByTitle("sentMessages.email.htmlFrame").getAttribute("srcdoc")
        expect(srcdoc?.startsWith(CSP)).toBe(true)
        expect(srcdoc).not.toMatch(/refresh/i)
        expect(srcdoc).not.toMatch(/<base/i)
        expect(srcdoc).toContain("<b>845120</b>")
    })

    it("R6-07: keeps one chat per sender and recipient", () => {
        const [first] = channelMessages(EMessageChannel.WHATSAPP)
        const other = {...first, id: "fake-other", sender: "209876543210"}
        const chats = conversations([first, other])
        expect(chats).toHaveLength(2)
        expect(chats.map(({sender}) => sender).sort()).toEqual(["109876543210", "209876543210"])
    })
})

describe("fix round 3 (R7)", () => {
    it("R7-21: links in an email are inert text that shows where they lead", () => {
        const srcdoc = emailSrcDoc(
            '<p>Open <a href="https://vote.example/login?ref=1" target="_blank">the portal</a>' +
                ' or <a href="https://vote.example/help">https://vote.example/help</a>.</p>' +
                '<a name="top">top</a>' +
                '<img usemap="#m"><map name="m"><area href="https://evil.example/" shape="rect"></map>' +
                '<svg><a href="https://evil.example/svg" xlink:href="https://evil.example/x"><text>s</text></a></svg>'
        )
        const frame = new DOMParser().parseFromString(srcdoc, "text/html")
        expect(frame.querySelectorAll("[href], [xlink\\:href]")).toHaveLength(0)
        expect(
            Array.from(frame.querySelectorAll("*")).some((e) => e.hasAttribute("xlink:href"))
        ).toBe(false)
        expect(frame.body.textContent).toContain("the portal (https://vote.example/login?ref=1)")
        // A link whose text is its address isn't shown twice.
        expect(frame.body.textContent).toContain("or https://vote.example/help.")
        expect(frame.body.textContent).not.toContain("help (https://vote.example/help)")
        expect(frame.body.textContent).toContain("top")
    })

    it("R7-20: maps every reason the fake provider shows, and none it no longer writes", () => {
        const fake = readFileSync(
            resolve(__dirname, "../../../../../harvest/src/routes/fake_provider.rs"),
            "utf8"
        )
        const written = Object.fromEntries(
            Array.from(
                fake.matchAll(/pub const (\w+)_REASON: &str =\s*"([^"]+)"/g),
                (match) => [match[1], match[2]] as const
            )
        )
        // Not shown as a reason: a report with no receipts is shown by its
        // state, and a message gone from the list as a failed read.
        const NOT_SHOWN = ["NO_RECEIPTS", "MESSAGE_GONE"]
        expect(Object.keys(written).length).toBeGreaterThan(4)
        expect(Object.keys(FAKE_PROVIDER_REASONS).sort()).toEqual(
            Object.entries(written)
                .filter(([name]) => !NOT_SHOWN.includes(name))
                .map(([, text]) => text)
                .sort()
        )
    })
})

describe("Demo approved message copy", () => {
    const translator = jest.requireActual<typeof import("i18next")>("i18next").createInstance()
    beforeAll(async () => {
        await translator.init({
            lng: "en",
            fallbackLng: "en",
            showSupportNotice: false,
            resources: {
                en: {translation: jest.requireActual("@/translations/en").default.translations},
                es: {translation: jest.requireActual("@/translations/es").default.translations},
            },
            interpolation: {escapeValue: false},
        })
    })
    it("renders complete voting copy from the approved demo parameters", () => {
        const message = {
            ...fullMessage("fake-2")!.content,
            text: null,
            parameters: [
                {name: null, value: "Ana"},
                {name: null, value: "Council Election 2026"},
                {name: null, value: "https://vote.example.org/council"},
            ],
        }
        expect(demoMessageText(message, translator.t)).toBe(
            "Voting reminder\n\nHello Ana, voting is now open for Council Election 2026.\n\nSign in to the voting portal to review your ballot and cast your vote:\nhttps://vote.example.org/council\n\nDo not share your sign-in code with anyone. If you have already voted, no further action is needed."
        )
        expect(demoMessageText({...message, text: "Captured original copy"}, translator.t)).toBe(
            "Captured original copy"
        )
        expect(
            demoMessageText({...message, template_name: "another_provider_template"}, translator.t)
        ).toBeNull()
        expect(
            demoMessageText({...message, parameters: [{name: null, value: "Ana"}]}, translator.t)
        ).toBeNull()
    })
    it("uses the captured language for verification copy and supports locale codes", () => {
        const content = fullMessage("fake-1")!.content
        expect(demoMessageText({...content, template_language: "en_US"}, translator.t)).toBe(
            "482913 is your verification code. For your security, do not share this code with anyone."
        )
        expect(demoMessageText({...content, template_language: "es"}, translator.t)).toBe(
            "482913 es su código de verificación. Por su seguridad, no lo comparta con nadie."
        )
        expect(demoMessageText({...content, code: null}, translator.t)).toBeNull()
    })
    it("reads old two-parameter reminders without treating a post name as a portal URL", () => {
        const content = fullMessage("fake-2")!.content
        const old = {
            ...content,
            parameters: [
                {name: null, value: "Ana"},
                {name: null, value: "https://vote.example.org/council"},
            ],
        }
        expect(demoMessageText(old, translator.t)).toContain("Hello Ana, voting is now open.")
        expect(demoMessageText(old, translator.t)).toContain("https://vote.example.org/council")
        expect(
            demoMessageText(
                {
                    ...old,
                    parameters: [
                        {name: null, value: "Ana"},
                        {name: "post", value: "Manila"},
                    ],
                },
                translator.t
            )
        ).toBeNull()
    })
})
