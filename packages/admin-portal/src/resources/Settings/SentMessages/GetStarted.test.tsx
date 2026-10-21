/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {useMutation} from "@apollo/client"
import {EMessageChannel} from "@/types/messaging"
import {ChannelChat} from "./ChannelChat"
import {FAKE_PROVIDER_GET_STARTED} from "./queries"
import {useFakeGetStarted} from "./useSentMessages"
import {MESSENGER_LINK, channelMessages} from "./__stories__/SentMessagesFixture"

jest.mock("@apollo/client", () => ({
    gql: jest.requireActual("@apollo/client").gql,
    useQuery: jest.fn(),
    useMutation: jest.fn(),
    NetworkStatus: {refetch: 4},
}))
// The key, then any values, so a test can read what the text was built from.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values
                ? `${key}(${Object.entries(values)
                      .map(([name, value]) => `${name}=${value}`)
                      .join(",")})`
                : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@/providers/AuthContextProvider", () => {
    const {createContext} = jest.requireActual("react")
    return {AuthContext: createContext({isAuthorized: () => true})}
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

const useMutationMock = useMutation as jest.Mock
let getStarted: jest.Mock

/** The Messenger app with its Get Started box, wired as MessagingTabs wires it. */
const Messenger: React.FC = () => {
    const onGetStarted = useFakeGetStarted()
    return (
        <ChannelChat
            channel={EMessageChannel.MESSENGER}
            messages={channelMessages(EMessageChannel.MESSENGER)}
            onGetStarted={onGetStarted}
        />
    )
}

const box = () => screen.getByRole("form", {name: "sentMessages.getStarted.title"})
const linkField = () => within(box()).getByRole("textbox", {name: "sentMessages.getStarted.link"})
const submitButton = () =>
    within(box()).getByRole("button", {name: "sentMessages.getStarted.submit"})

/** Pastes `link` and presses Get Started; resolves once the answer is shown. */
const pasteAndSubmit = async (link: string) => {
    fireEvent.change(linkField(), {target: {value: link}})
    await act(async () => {
        fireEvent.click(submitButton())
    })
}

/** A refusal as Hasura forwards harvest's message. */
const refusal = (message: string) => ({graphQLErrors: [{message}]})

beforeEach(() => {
    getStarted = jest.fn()
    useMutationMock.mockReset()
    useMutationMock.mockReturnValue([getStarted])
})

describe("the Messenger Get Started box", () => {
    it("sends the pasted link, trimmed, as `link` with the messaging-account-write role", async () => {
        getStarted.mockResolvedValue({data: {messaging_fake_provider_get_started: {status: 200}}})
        render(<Messenger />)
        await pasteAndSubmit(`  ${MESSENGER_LINK}\n`)

        expect(useMutationMock).toHaveBeenCalledWith(FAKE_PROVIDER_GET_STARTED, {
            context: {headers: {"x-hasura-role": "messaging-account-write"}},
        })
        expect(getStarted).toHaveBeenCalledTimes(1)
        expect(getStarted).toHaveBeenCalledWith({variables: {link: MESSENGER_LINK}})
    })

    it("shows that the webhook component accepted the event (200)", async () => {
        getStarted.mockResolvedValue({data: {messaging_fake_provider_get_started: {status: 200}}})
        render(<Messenger />)
        await pasteAndSubmit(MESSENGER_LINK)

        expect(within(box()).getByRole("status")).toHaveTextContent(
            "sentMessages.getStarted.accepted"
        )
    })

    it("shows any other status the webhook component answered, with its reason", async () => {
        getStarted.mockResolvedValue({
            data: {
                messaging_fake_provider_get_started: {
                    status: 503,
                    reason: "The webhook component didn't answer",
                },
            },
        })
        render(<Messenger />)
        await pasteAndSubmit(MESSENGER_LINK)

        const status = within(box()).getByRole("status")
        expect(status).toHaveTextContent("sentMessages.getStarted.answered(status=503)")
        expect(status).toHaveTextContent("sentMessages.reason.UNREACHABLE")
    })

    it("shows the action error's reason when harvest refuses the link", async () => {
        getStarted.mockRejectedValue(refusal("The link has no ref: copy the whole link"))
        render(<Messenger />)
        await pasteAndSubmit("https://m.me/council.page")

        expect(within(box()).getByRole("status")).toHaveTextContent(
            "sentMessages.getStarted.failed(reason=sentMessages.reason.LINK_WITHOUT_REF)"
        )
    })

    it("says there's no reason when the action answers nothing", async () => {
        getStarted.mockResolvedValue({data: {messaging_fake_provider_get_started: null}})
        render(<Messenger />)
        await pasteAndSubmit(MESSENGER_LINK)

        expect(within(box()).getByRole("status")).toHaveTextContent(
            "sentMessages.getStarted.failed(reason=sentMessages.report.noReason)"
        )
    })

    it("can't be pressed while the link is empty or only spaces", () => {
        render(<Messenger />)
        expect(submitButton()).toBeDisabled()
        fireEvent.change(linkField(), {target: {value: "   "}})
        expect(submitButton()).toBeDisabled()
        fireEvent.click(submitButton())
        expect(getStarted).not.toHaveBeenCalled()
        expect(within(box()).queryByRole("status")).not.toBeInTheDocument()
    })

    it("sends a link that isn't m.me as it is and shows harvest's refusal", async () => {
        getStarted.mockRejectedValue(refusal("Paste the m.me link from the enrollment page"))
        render(<Messenger />)
        await pasteAndSubmit("https://example.org/start")

        // The box doesn't check the link itself; harvest does.
        expect(getStarted).toHaveBeenCalledWith({variables: {link: "https://example.org/start"}})
        expect(within(box()).getByRole("status")).toHaveTextContent(
            "sentMessages.getStarted.failed(reason=sentMessages.reason.LINK_UNREADABLE)"
        )
    })
})
