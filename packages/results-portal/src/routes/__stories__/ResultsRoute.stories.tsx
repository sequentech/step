// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {useTranslation} from "react-i18next"
import initSqlJs from "sql.js"
import {routeFetch} from "@sequentech/ui-test-kit/adapters/fetch"
import {S3Mock} from "@sequentech/ui-test-kit/mocks/s3"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"
import {App} from "@/App"
import {ResultsRoute} from "@/routes/ResultsRoute"
import {SettingsContext, type GlobalSettings} from "@/providers/SettingsContextProvider"
import {CustomCssContextProvider} from "@/providers/CustomCssContextProvider"
import {
    ResultsManifestContextProvider,
    useResultsManifest,
} from "@/providers/ResultsManifestContextProvider"
import {ResultsAuthContextProvider} from "@/providers/ResultsAuthContextProvider"
import {resultsFixture, resultsIds} from "../../../tests/fixtures/results"
import {exportDataset} from "../../../tests/fixtures/sqliteDataset"
import {
    EStoryLocale,
    EStoryTenant,
    STORY_BRANDING,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"
import {EStoryDataState, pending} from "../../../../ui-essentials/.storybook/screens"

const S3_ORIGIN = "https://s3.story.test"
const INDEX = `results-index/${resultsIds.event}.json`
const settings: GlobalSettings = {
    RESULTS_PORTAL_CLIENT_ID: "results-portal",
    KEYCLOAK_URL: "https://keycloak.story.test/",
    HASURA_URL: "https://hasura.story.test/v1/graphql",
    APP_VERSION: "story",
    APP_HASH: "story",
    PUBLIC_BUCKET_URL: new S3Mock({
        origin: S3_ORIGIN,
        violations: new ViolationLog(),
    }).publicBucketUrl(),
}

/** Selects the toolbar locale once the publication offers it, as a voter does in the header. */
function PublicationLocale() {
    const {locale} = useStoryGlobals()
    const {availableLanguages} = useResultsManifest()
    const {i18n} = useTranslation()
    useEffect(() => {
        if (availableLanguages.includes(locale) && i18n.language !== locale) {
            void i18n.changeLanguage(locale)
        }
    }, [availableLanguages, i18n, locale])
    return null
}

/** The portal's providers and application shell around the results route. */
function ResultsShell() {
    return (
        <SettingsContext.Provider value={{loaded: true, globalSettings: settings}}>
            <CustomCssContextProvider>
                <ResultsManifestContextProvider>
                    <ResultsAuthContextProvider>
                        <PublicationLocale />
                        <App />
                    </ResultsAuthContextProvider>
                </ResultsManifestContextProvider>
            </CustomCssContextProvider>
        </SettingsContext.Provider>
    )
}

const publicationDefects = {
    expectedFailure: {
        reason: "Selected tabs are MUI blue on the page background at 4.33 contrast, and the shared participation summary renders an empty column header.",
        a11y: ["color-contrast", "empty-table-header"],
    },
}

interface Scenario {
    data: EStoryDataState
}

const meta = {
    title: "Screens/Results/Publication",
    args: {data: EStoryDataState.POPULATED},
    parameters: {
        router: {
            parentPath: "/",
            layout: ResultsShell,
            path: ":eeId",
            initialEntries: [`/${resultsIds.event}`],
        },
    },
    loaders: [
        async () => {
            const sql = await initSqlJs({locateFile: (file) => `/${file}`})
            return {sqlite: exportDataset(sql, resultsFixture().dataset)}
        },
    ],
    beforeEach: ({args, globals, loaded}) => {
        const {sqlite} = loaded as {sqlite: Uint8Array}
        const {tenant} = readStoryGlobals(globals)
        const {index, manifest} = resultsFixture()
        const violations = new ViolationLog()
        const s3 = new S3Mock({origin: S3_ORIGIN, violations})
        s3.putJson("public", INDEX, index)
        s3.putJson("public", "results/manifest.json", {
            ...manifest,
            available_languages: Object.values(EStoryLocale),
            custom_css: {election_event: STORY_BRANDING[tenant].css ?? null},
        })
        s3.putBytes("public", "results/full.sqlite", sqlite, "application/vnd.sqlite3")
        if (args.data === EStoryDataState.EMPTY)
            s3.override(({key}) => key === INDEX, {status: 404})
        if (args.data === EStoryDataState.ERROR)
            s3.override(({key}) => key === INDEX, {status: 500})
        const restore = routeFetch(
            [
                {
                    handles: (url) => s3.handles(url),
                    handle: (request) =>
                        args.data === EStoryDataState.LOADING ? pending() : s3.handle(request),
                },
            ],
            violations
        )
        return () => {
            restore()
            expect(violations.list()).toEqual([])
        }
    },
    render: (_args, {globals}) => <ResultsRoute key={JSON.stringify(globals)} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>

export const Loading: Story = {
    args: {data: EStoryDataState.LOADING},
    play: async ({canvasElement}) => {
        const main = within(await within(canvasElement).findByRole("main"))
        await expect(main.queryByRole("heading", {level: 1})).not.toBeInTheDocument()
        await expect(main.queryByRole("table")).not.toBeInTheDocument()
    },
}

export const Empty: Story = {
    args: {data: EStoryDataState.EMPTY},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {name: "Results not published yet"})
        ).toBeVisible()
        await expect(
            canvas.getByText("Results are not available at this time. Please check back later.")
        ).toBeVisible()
    },
}

export const Populated: Story = {
    parameters: publicationDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {name: "Community Election Results"})
        ).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("45")
    },
}

export const LoadError: Story = {
    args: {data: EStoryDataState.ERROR},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {name: "Unexpected error"})).toBeVisible()
        await expect(
            canvas.getByText(
                "We could not load results right now. Please try again in a few minutes."
            )
        ).toBeVisible()
    },
}

export const CustomBranding: Story = {
    parameters: publicationDefects,
    globals: {tenant: EStoryTenant.CUSTOM},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const heading = await canvas.findByRole("heading", {name: "Community Election Results"})
        await waitFor(() => expect(getComputedStyle(heading).color).toBe("rgb(11, 79, 58)"))
    },
}
