// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useLayoutEffect, useState} from "react"

const inertBackgrounds = new WeakMap<Element, {count: number; wasInert: boolean}>()

const makeBackgroundInert = (modal: HTMLElement) => {
    const siblings = Array.from(modal.parentElement?.children ?? []).filter(
        (element) =>
            element !== modal &&
            !element.matches(".MuiModal-hidden, .MuiModal-root:not([aria-hidden='true'])")
    )
    siblings.forEach((element) => {
        const state = inertBackgrounds.get(element) ?? {
            count: 0,
            wasInert: element.hasAttribute("inert"),
        }
        state.count += 1
        inertBackgrounds.set(element, state)
        element.setAttribute("inert", "")
    })
    return () => {
        siblings.forEach((element) => {
            const state = inertBackgrounds.get(element)
            if (!state || --state.count > 0) {
                return
            }
            if (!state.wasInert) {
                element.removeAttribute("inert")
            }
            inertBackgrounds.delete(element)
        })
    }
}

export const useInertBackground = (open: boolean) => {
    const [modalRoot, setModalRoot] = useState<HTMLElement | null>(null)

    // Release inert before MUI restores focus, including when overlays overlap.
    useLayoutEffect(() => {
        if (open && modalRoot) {
            return makeBackgroundInert(modalRoot)
        }
    }, [open, modalRoot])

    return setModalRoot
}
