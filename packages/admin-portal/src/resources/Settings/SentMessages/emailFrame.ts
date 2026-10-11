// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** Nothing loads from outside the email: no remote images, styles, fonts or frames. */
export const EMAIL_CSP = "default-src 'none'; img-src data:; style-src 'unsafe-inline'"

const CSP_META = `<meta http-equiv="Content-Security-Policy" content="${EMAIL_CSP}">`

/**
 * The `srcDoc` of the reading pane's sandboxed iframe (R6-06).
 *
 * - The CSP meta is the very first markup: a CSP meta governs only what is
 *   parsed after it, and the parser hoists it into `<head>` even before the
 *   email's own `<html><head>`.
 * - `<meta http-equiv="refresh">` and `<base>` are removed: neither the
 *   sandbox nor the CSP stops a refresh from navigating the frame itself.
 * - Links don't navigate (R7-21): a click on a link would load the page
 *   inside the frame. Each HTML link becomes inert text followed by where it
 *   leads, `text (https://…)`, so staff still see the address (unless the
 *   text already is the address). Any other element that links (an image
 *   map's `<area>`, an SVG `<a>`) loses its link.
 *
 * DOMParser documents are inert: nothing runs or loads while parsing.
 */
export const emailSrcDoc = (html: string): string => {
    const document = new DOMParser().parseFromString(html, "text/html")
    for (const element of Array.from(document.querySelectorAll("base, meta[http-equiv]"))) {
        const equiv = element.getAttribute("http-equiv")?.trim().toLowerCase()
        if (element.tagName === "BASE" || equiv === "refresh") {
            element.remove()
        }
    }
    inertLinks(document)
    return CSP_META + document.documentElement.outerHTML
}

const HTML_NAMESPACE = "http://www.w3.org/1999/xhtml"
const XLINK_NAMESPACE = "http://www.w3.org/1999/xlink"

const inertLinks = (document: Document) => {
    for (const element of Array.from(document.querySelectorAll("*"))) {
        const href = element.getAttribute("href")
        const isHtmlLink = element.namespaceURI === HTML_NAMESPACE && element.localName === "a"
        if (isHtmlLink && href !== null) {
            const text = document.createElement("span")
            text.append(...Array.from(element.childNodes))
            const address = href.trim()
            if (address && text.textContent?.trim() !== address) {
                text.append(` (${address})`)
            }
            element.replaceWith(text)
            continue
        }
        element.removeAttribute("href")
        element.removeAttributeNS(XLINK_NAMESPACE, "href")
    }
}
