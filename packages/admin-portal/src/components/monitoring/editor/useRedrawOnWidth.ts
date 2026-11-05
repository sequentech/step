// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useRef} from "react"
import type {YamlDraftController} from "./yamlDraft"

/**
 * Draws the preview again when the width it is drawn at changes: the pane
 * it shows in is measured only once the document has loaded, after its
 * first render was asked for. A document still loading is left alone, since
 * the render its load asks for already takes the new width.
 */
export const useRedrawOnWidth = (
    controller: YamlDraftController,
    width: number,
    ready: boolean
) => {
    const drawnAt = useRef(width)
    useEffect(() => {
        if (drawnAt.current === width) return
        drawnAt.current = width
        // `ready` changing alone is the load, which draws the preview itself.
        if (ready) void controller.previewNow()
    }, [controller, width, ready])
}
