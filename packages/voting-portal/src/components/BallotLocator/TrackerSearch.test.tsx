/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {policyIdDisplay} from "@vitrine/verify"
import {typedBallotId} from "../../services/vitrine/ballotId"
import {TrackerSearch} from "./TrackerSearch"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@sequentech/ui-core", () => ({
    translateHtml: (_t: unknown, key: string) => key,
}))

const POLICY_ID = `sha256:${"0123456789abcdef".repeat(4)}`
const FOUR_GROUPS = policyIdDisplay(POLICY_ID).split("-").slice(0, 4).join("-")

const Field: React.FC<{onSubmit: () => void; policyId?: string}> = ({onSubmit, policyId}) => {
    const [value, setValue] = useState("")
    return (
        <TrackerSearch
            value={value}
            onChange={setValue}
            onSubmit={onSubmit}
            canSubmit={typedBallotId(value).replace(/-/g, "").length >= 8}
            policyId={policyId}
        />
    )
}

const field = () => screen.getByRole("textbox") as HTMLInputElement
const find = () => screen.getByRole("button", {name: /ballotLocator\.search\.find/})

const paste = async (text: string) => {
    await userEvent.click(field())
    await userEvent.paste(text)
}

describe("the tracker's search when it gets the election's policy ID", () => {
    it("says what it is, and finds no ballot", async () => {
        const onSubmit = jest.fn()
        render(<Field onSubmit={onSubmit} policyId={POLICY_ID} />)

        await paste(FOUR_GROUPS)
        expect(field().value).toBe(FOUR_GROUPS)
        const answer = screen.getByRole("status")
        expect(answer).toHaveTextContent("ballotLocator.policyId")
        expect(field()).toHaveAttribute("aria-describedby", answer.id)
        expect(screen.queryByText(/ballotLocator\.search\.chips\./)).toBeNull()
        expect(find()).toBeDisabled()
        await userEvent.type(field(), "{Enter}")
        expect(onSubmit).not.toHaveBeenCalled()
    })

    it("answers for the whole ID too", async () => {
        render(<Field onSubmit={jest.fn()} policyId={POLICY_ID} />)
        await paste(policyIdDisplay(POLICY_ID))
        expect(screen.getByRole("status")).toHaveTextContent("ballotLocator.policyId")
    })

    it("leaves a short Tracking ID to search, even with the policy known", async () => {
        const onSubmit = jest.fn()
        render(<Field onSubmit={onSubmit} policyId={POLICY_ID} />)

        await paste(FOUR_GROUPS.split("-").slice(0, 2).join("-"))
        expect(screen.queryByRole("status")).toBeNull()
        expect(screen.getByText(/ballotLocator\.search\.chips\.short/)).toBeVisible()
        expect(find()).toBeEnabled()
        await userEvent.type(field(), "{Enter}")
        expect(onSubmit).toHaveBeenCalledTimes(1)
    })

    it("searches the text as a Tracking ID when the election has no policy", async () => {
        const onSubmit = jest.fn()
        render(<Field onSubmit={onSubmit} />)

        await paste(FOUR_GROUPS)
        expect(screen.queryByRole("status")).toBeNull()
        expect(find()).toBeEnabled()
    })
})
