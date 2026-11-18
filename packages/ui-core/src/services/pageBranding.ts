// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect} from "react"
import {
    EPlatformBrandingPolicy,
    IElectionEventPresentation,
} from "../types/ElectionEventPresentation"

type Presentation = IElectionEventPresentation | null | undefined

export const PLATFORM_BRANDING_HIDDEN_CLASS = "platform-branding-hidden"

export const hidesPlatformBranding = (presentation: Presentation): boolean =>
    presentation?.platform_branding_policy === EPlatformBrandingPolicy.HIDE

export interface EventLogoDefaults {
    platform: string
    blank: string
}

// No logo is shown while the event is unknown; the platform's own logo is the
// default only once the event is known to have none and to show the platform.
export const eventLogo = (presentation: Presentation, defaults: EventLogoDefaults): string => {
    if (presentation?.logo_url) {
        return presentation.logo_url
    }
    return presentation?.logo_url === null && !hidesPlatformBranding(presentation)
        ? defaults.platform
        : defaults.blank
}

export interface PageHeadLink {
    rel: string
    href: string
    type?: string
    sizes?: string
}

const NO_ICON: PageHeadLink[] = [{rel: "icon", href: "data:,"}]

const PLATFORM_HEAD_LINKS: PageHeadLink[] = [
    {rel: "icon", type: "image/png", href: "/favicon-96x96.png", sizes: "96x96"},
    {rel: "icon", type: "image/svg+xml", href: "/favicon.svg"},
    {rel: "shortcut icon", href: "/favicon.ico"},
    {rel: "apple-touch-icon", sizes: "180x180", href: "/apple-touch-icon.png"},
    {rel: "manifest", href: "/site.webmanifest"},
]

// Relative addresses are served by the portal itself, so any web base will do.
const RELATIVE_URL_BASE = "https://portal.invalid/"

const isWebUrl = (url: string): boolean => {
    try {
        return ["http:", "https:"].includes(new URL(url, RELATIVE_URL_BASE).protocol)
    } catch {
        return false
    }
}

// The tab shows no icon until the event is known, then the event's own icon,
// or the platform's unless the event hides the platform's branding.
export const pageHeadLinks = (presentation: Presentation): PageHeadLink[] => {
    if (!presentation) {
        return NO_ICON
    }
    const faviconUrl = presentation.favicon_url?.trim()
    if (faviconUrl && isWebUrl(faviconUrl)) {
        return [{rel: "icon", href: faviconUrl}]
    }
    return hidesPlatformBranding(presentation) ? NO_ICON : PLATFORM_HEAD_LINKS
}

const HEAD_LINK_MARKER = "data-page-branding"

const setPageHead = (page: Document, title: string, links: PageHeadLink[]): void => {
    page.title = title
    page.head.querySelectorAll(`link[${HEAD_LINK_MARKER}]`).forEach((link) => link.remove())
    for (const spec of links) {
        const link = page.createElement("link")
        link.setAttribute(HEAD_LINK_MARKER, "")
        Object.entries(spec).forEach(([name, value]) => link.setAttribute(name, value))
        page.head.appendChild(link)
    }
}

const pageTitle = (presentation: Presentation, defaultTitle: string): string =>
    presentation?.page_title?.trim() || defaultTitle

export const applyPageBranding = (
    page: Document,
    presentation: Presentation,
    defaultTitle: string
): void => setPageHead(page, pageTitle(presentation, defaultTitle), pageHeadLinks(presentation))

const defaultPageTitle = typeof document === "undefined" ? "" : document.title

export const usePageBranding = (presentation: Presentation): void => {
    const title = pageTitle(presentation, defaultPageTitle)
    const links = JSON.stringify(pageHeadLinks(presentation))

    useEffect(() => {
        setPageHead(document, title, JSON.parse(links) as PageHeadLink[])
    }, [title, links])
}
