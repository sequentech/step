// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {BrowserContext} from "@playwright/test"

/** Restore element type checks in Obscura 0.2.2 without changing application requests. */
export async function installObscuraCompatibility(context: BrowserContext): Promise<void> {
    await context.addInitScript(() => {
        // Obscura aliases these constructors to Element. Consequently <head>
        // passes instanceof HTMLIFrameElement, and style-loader tries to insert
        // styles into a nonexistent iframe document. See obscura issue #817.
        const interfaces: Record<string, string[]> = {
            HTMLIFrameElement: ["iframe"],
            HTMLInputElement: ["input"],
            HTMLSelectElement: ["select"],
            HTMLTextAreaElement: ["textarea"],
            HTMLButtonElement: ["button"],
            HTMLFormElement: ["form"],
            HTMLImageElement: ["img"],
            HTMLScriptElement: ["script"],
            HTMLStyleElement: ["style"],
            HTMLLinkElement: ["link"],
            HTMLAnchorElement: ["a"],
            HTMLDivElement: ["div"],
            HTMLSpanElement: ["span"],
            HTMLHeadElement: ["head"],
            HTMLBodyElement: ["body"],
            HTMLHtmlElement: ["html"],
            HTMLOptionElement: ["option"],
            HTMLLabelElement: ["label"],
            HTMLCanvasElement: ["canvas"],
            HTMLVideoElement: ["video"],
            HTMLAudioElement: ["audio"],
            HTMLMediaElement: ["audio", "video"],
        }
        const globals = globalThis as unknown as Record<string, unknown>
        for (const [name, tags] of Object.entries(interfaces)) {
            if (globals[name] !== Element) continue
            const replacement = function () {
                throw new TypeError("Illegal constructor")
            }
            replacement.prototype = Element.prototype
            Object.defineProperty(replacement, "name", {value: name})
            Object.defineProperty(replacement, Symbol.hasInstance, {
                value: (value: unknown) =>
                    value instanceof Element && tags.includes(value.tagName.toLowerCase()),
            })
            globals[name] = replacement
        }
    })
}
