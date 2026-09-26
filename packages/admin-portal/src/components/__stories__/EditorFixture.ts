// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// TinyMCE loads from the Storybook server's /tinymce directory, as the admin
// application does from its public directory, and edits inside an iframe.
import {expect, within} from "storybook/test"
import type {TinyMCE} from "tinymce"

/** Waits until TinyMCE has loaded and returns its editable document body. */
export async function editorBody(container: HTMLElement) {
    await expect(
        await within(container).findByRole("button", {name: "Bold"}, {timeout: 10_000})
    ).toBeVisible()
    const frame = container.querySelector<HTMLIFrameElement>("iframe.tox-edit-area__iframe")
    const body = frame?.contentDocument?.body
    if (!body) throw new Error("TinyMCE did not create its editing frame")
    return body
}

/**
 * TinyMCE ran from the Storybook server's /tinymce directory: it loads its
 * theme, plugins and skins relative to that base once per page, and the
 * story's network guard fails any other request.
 */
export function expectLocalTinymce() {
    const script = document.querySelector<HTMLScriptElement>('script[src$="/tinymce.min.js"]')
    expect(script?.src).toBe(`${location.origin}/tinymce/tinymce.min.js`)
    const {tinymce} = window as Window & {tinymce?: TinyMCE}
    expect(tinymce?.baseURL).toBe(`${location.origin}/tinymce`)
}

/** TinyMCE's own status bar, present in every editor of the admin portal. */
export const TINYMCE_DEFECTS = {
    reason: "TinyMCE's status bar resize handle is a div with an aria-label and no role.",
    a11y: ["aria-prohibited-attr"],
}
