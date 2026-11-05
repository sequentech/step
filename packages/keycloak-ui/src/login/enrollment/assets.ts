// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The widgets sequent-theme ships as plain scripts (intl-tel-input, zxcvbn):
// the React themes inherit them and load them where a field needs one.
const scripts = new Map<string, Promise<void>>()

export enum ScriptKind {
    Classic = "CLASSIC",
    Module = "MODULE",
}

export function loadScript(src: string, kind = ScriptKind.Classic): Promise<void> {
    let loading = scripts.get(src)
    if (loading === undefined) {
        loading = new Promise<void>((resolve, reject) => {
            const script = document.createElement("script")
            if (kind === ScriptKind.Module) script.type = "module"
            script.src = src
            script.onload = () => resolve()
            script.onerror = () => {
                scripts.delete(src)
                script.remove()
                reject(new Error(`Could not load ${src}`))
            }
            document.head.append(script)
        })
        scripts.set(src, loading)
    }
    return loading
}

export function loadStylesheet(href: string): void {
    for (const link of document.head.querySelectorAll("link[rel='stylesheet']")) {
        if (link.getAttribute("href") === href) return
    }
    const link = document.createElement("link")
    link.rel = "stylesheet"
    link.href = href
    document.head.append(link)
}
