// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext} from "react"
import type {Decorator} from "@storybook/react-vite"
import type {Globals, GlobalTypes} from "storybook/internal/types"

/** Languages ui-core registers for every portal; the toolbar lists them in its order. */
export enum EStoryLocale {
    ENGLISH = "en",
    SPANISH = "es",
    CATALAN = "cat",
    FRENCH = "fr",
    TAGALOG = "tl",
    GALICIAN = "gl",
    DUTCH = "nl",
    BASQUE = "eu",
}

/**
 * Branding a tenant or election event configures: the portals only vary the
 * logo and a custom CSS string, never the MUI theme.
 */
export enum EStoryTenant {
    /** No custom CSS; the Sequent logo (`logo_url: null`). */
    SEQUENT = "sequent",
    /** No logo configured (`logo_url` absent): voting and verifier show a blank logo. */
    UNBRANDED = "unbranded",
    /** A synthetic logo and custom CSS. */
    CUSTOM = "custom",
}

/** Role groups of the default tenant realm template; `none` holds no roles. */
export enum EStoryPermissions {
    ADMIN = "admin",
    ADMIN_LIGHT = "admin-light",
    ADMIN_LOCKDOWN = "admin-lockdown",
    TRUSTEE = "trustee",
    NONE = "none",
}

/**
 * Election event lifecycle, following the steps of the admin dashboard, with
 * the tally ceremony between the end of voting and the results.
 */
export enum EStoryWorkflow {
    /** Keys ceremony in progress. */
    CREATED = "created",
    /** Keys ceremony completed; elections not published. */
    KEYS = "keys",
    /** Elections published; voting not started. */
    PUBLISHED = "published",
    /** Voting open. */
    STARTED = "started",
    /** Voting closed. */
    ENDED = "ended",
    /** Tally ceremony in progress. */
    TALLY = "tally",
    /** Tally completed and results published. */
    RESULTS = "results",
}

export interface StoryGlobals {
    locale: EStoryLocale
    tenant: EStoryTenant
    permissions: EStoryPermissions
    workflow: EStoryWorkflow
}

export const DEFAULT_STORY_GLOBALS: StoryGlobals = {
    locale: EStoryLocale.ENGLISH,
    tenant: EStoryTenant.SEQUENT,
    permissions: EStoryPermissions.ADMIN,
    workflow: EStoryWorkflow.ENDED,
}

const member = <T extends string>(values: Record<string, T>, value: unknown, fallback: T): T =>
    Object.values(values).find((candidate) => candidate === value) ?? fallback

/** Typed globals; values from an unknown URL or an older link fall back to the defaults. */
export const readStoryGlobals = (globals: Globals): StoryGlobals => ({
    locale: member(EStoryLocale, globals.locale, DEFAULT_STORY_GLOBALS.locale),
    tenant: member(EStoryTenant, globals.tenant, DEFAULT_STORY_GLOBALS.tenant),
    permissions: member(EStoryPermissions, globals.permissions, DEFAULT_STORY_GLOBALS.permissions),
    workflow: member(EStoryWorkflow, globals.workflow, DEFAULT_STORY_GLOBALS.workflow),
})

const CUSTOM_LOGO =
    '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="40" viewBox="0 0 200 40">' +
    '<rect width="40" height="40" rx="8" fill="#0b6e4f"/>' +
    '<text x="50" y="27" font-family="sans-serif" font-size="18" fill="#0b4f3a">' +
    "Example Council</text></svg>"

/** What a portal receives as `logo_url` and `css` in a presentation or tenant annotations. */
export interface StoryBranding {
    /** Null selects the Sequent logo; an absent value selects the blank placeholder. */
    logo_url?: string | null
    css?: string
}

export const STORY_BRANDING: Record<EStoryTenant, StoryBranding> = {
    [EStoryTenant.SEQUENT]: {logo_url: null},
    [EStoryTenant.UNBRANDED]: {},
    [EStoryTenant.CUSTOM]: {
        logo_url: `data:image/svg+xml,${encodeURIComponent(CUSTOM_LOGO)}`,
        css: [
            ".header-class { background-color: #e6f4ec; border-bottom: 4px solid #0b6e4f; }",
            'h1, h2, h3, h4, h5, h6, [class*="MuiTypography-h"] { color: #0b4f3a; }',
        ].join("\n"),
    },
}

const items = <T extends string>(titles: Record<T, string>) =>
    (Object.entries(titles) as [T, string][]).map(([value, title]) => ({value, title}))

/**
 * Toolbar definitions. A Storybook lists only the globals its stories map to
 * their fixtures; `readStoryGlobals` gives stories the defaults of the others.
 */
export const storyGlobalTypes = {
    locale: {
        description: "Language of the translations",
        toolbar: {
            title: "Locale",
            icon: "globe",
            dynamicTitle: true,
            items: items<EStoryLocale>({
                [EStoryLocale.ENGLISH]: "English",
                [EStoryLocale.SPANISH]: "Spanish",
                [EStoryLocale.CATALAN]: "Catalan",
                [EStoryLocale.FRENCH]: "French",
                [EStoryLocale.TAGALOG]: "Tagalog",
                [EStoryLocale.GALICIAN]: "Galician",
                [EStoryLocale.DUTCH]: "Dutch",
                [EStoryLocale.BASQUE]: "Basque",
            }),
        },
    },
    tenant: {
        description: "Tenant branding: logo and custom CSS",
        toolbar: {
            title: "Tenant",
            icon: "paintbrush",
            dynamicTitle: true,
            items: items<EStoryTenant>({
                [EStoryTenant.SEQUENT]: "Sequent branding",
                [EStoryTenant.UNBRANDED]: "No logo",
                [EStoryTenant.CUSTOM]: "Custom logo and CSS",
            }),
        },
    },
    permissions: {
        description: "Role group of the signed-in administrator",
        toolbar: {
            title: "Role",
            icon: "key",
            dynamicTitle: true,
            items: items<EStoryPermissions>({
                [EStoryPermissions.ADMIN]: "Administrator",
                [EStoryPermissions.ADMIN_LIGHT]: "Administrator (light)",
                [EStoryPermissions.ADMIN_LOCKDOWN]: "Administrator (lockdown)",
                [EStoryPermissions.TRUSTEE]: "Trustee",
                [EStoryPermissions.NONE]: "No roles",
            }),
        },
    },
    workflow: {
        description: "Election event lifecycle step",
        toolbar: {
            title: "Workflow",
            icon: "timer",
            dynamicTitle: true,
            items: items<EStoryWorkflow>({
                [EStoryWorkflow.CREATED]: "Keys ceremony in progress",
                [EStoryWorkflow.KEYS]: "Keys ready",
                [EStoryWorkflow.PUBLISHED]: "Published",
                [EStoryWorkflow.STARTED]: "Voting open",
                [EStoryWorkflow.ENDED]: "Voting closed",
                [EStoryWorkflow.TALLY]: "Tally ceremony in progress",
                [EStoryWorkflow.RESULTS]: "Results published",
            }),
        },
    },
} satisfies Record<keyof StoryGlobals, GlobalTypes[string]>

const StoryGlobalsContext = createContext<StoryGlobals>(DEFAULT_STORY_GLOBALS)

/** The typed globals of the rendered story, for fixture components. */
export const useStoryGlobals = () => useContext(StoryGlobalsContext)

export const withStoryGlobals: Decorator = (Story, {globals}) => (
    <StoryGlobalsContext.Provider value={readStoryGlobals(globals)}>
        <Story />
    </StoryGlobalsContext.Provider>
)
