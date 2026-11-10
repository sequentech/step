// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderHook} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {useGetPublicDocumentUrl} from "./public-document-url"

jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {PUBLIC_BUCKET_URL: "https://files.invalid/"},
    }),
}))

const FOLDER = "https://files.invalid/tenant-north/document-document-1/"

const addressOf = (documentName: string) => {
    const {result} = renderHook(() => useGetPublicDocumentUrl(), {
        wrapper: ({children}: {children: React.ReactNode}) => (
            <MemoryRouter initialEntries={["/tenant/north/event/mayor/materials"]}>
                <Routes>
                    <Route path="/tenant/:tenantId/event/:eventId/*" element={children} />
                </Routes>
            </MemoryRouter>
        ),
    })
    return result.current.getDocumentUrl("document-1", documentName)
}

describe("useGetPublicDocumentUrl", () => {
    it("leaves a name of letters, digits, -, _ and . as it is", () => {
        expect(addressOf("voting-guide_2.pdf")).toBe(`${FOLDER}voting-guide_2.pdf`)
        expect(addressOf("ballot_receipt_mayor.pdf")).toBe(`${FOLDER}ballot_receipt_mayor.pdf`)
    })

    it("encodes spaces and percent signs", () => {
        expect(addressOf("voting guide.pdf")).toBe(`${FOLDER}voting%20guide.pdf`)
        expect(addressOf("100% turnout.pdf")).toBe(`${FOLDER}100%25%20turnout.pdf`)
    })

    it("encodes #, ? and ;", () => {
        expect(addressOf("results #2; final?.pdf")).toBe(
            `${FOLDER}results%20%232%3B%20final%3F.pdf`
        )
    })

    it.each(["notes #2.pdf", "what now?.pdf", "draft;1.pdf", "a#b?c;d%e.mp3"])(
        "keeps all of %p in the path, as its last segment",
        (documentName) => {
            const address = new URL(addressOf(documentName))
            expect(address.hash).toBe("")
            expect(address.search).toBe("")
            expect(address.pathname).not.toContain(";")
            const folder = "/tenant-north/document-document-1/"
            expect(address.pathname.startsWith(folder)).toBe(true)
            expect(decodeURIComponent(address.pathname.slice(folder.length))).toBe(documentName)
        }
    )

    it("keeps / as the separator of the name's segments", () => {
        expect(addressOf("guides/voting guide #1.pdf")).toBe(
            `${FOLDER}guides/voting%20guide%20%231.pdf`
        )
    })
})
