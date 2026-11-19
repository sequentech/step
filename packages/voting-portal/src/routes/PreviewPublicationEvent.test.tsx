// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, waitFor} from "@testing-library/react"
import {Provider} from "react-redux"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {store} from "../store/store"
import {PreviewPublicationEvent} from "./PreviewPublicationEvent"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {DISABLE_AUTH: false, PUBLIC_BUCKET_URL: "https://public.example/"},
    }),
}))

const PRESIGNED_URL = "https://storage.example/private/document/publication.json?X-Amz-Signature=s"

function open(path: string) {
    const router = createMemoryRouter(
        [
            {
                path: "/preview/:tenantId/:documentId/:areaId/:publicationId",
                element: <PreviewPublicationEvent />,
            },
        ],
        {initialEntries: [path]}
    )
    render(
        <Provider store={store}>
            <RouterProvider router={router} />
        </Provider>
    )
}

beforeEach(() => {
    sessionStorage.clear()
    global.fetch = jest.fn(() => Promise.resolve({ok: false, statusText: "Forbidden"})) as any
    jest.spyOn(console, "log").mockImplementation(() => undefined)
})

test("a private preview is read through the link in the fragment", async () => {
    open(`/preview/tenant/document/area/publication#${encodeURIComponent(PRESIGNED_URL)}`)
    await waitFor(() => expect(global.fetch).toHaveBeenCalledWith(PRESIGNED_URL))
})

test("a preview without a link is read from the public bucket", async () => {
    open("/preview/tenant/document/area/publication")
    await waitFor(() =>
        expect(global.fetch).toHaveBeenCalledWith(
            "https://public.example/tenant-tenant/document-document/publication.json"
        )
    )
})
