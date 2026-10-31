// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState, useSyncExternalStore} from "react"
import {YamlDraftController, type IYamlDraftOptions, type IYamlDraftState} from "./yamlDraft"

export interface IYamlDraft extends IYamlDraftState {
    controller: YamlDraftController
}

/**
 * One document being edited: the YAML text, what it parses to, the problems
 * found in it and its live preview. See {@link YamlDraftController}.
 *
 * `options.text` is read once; call `controller.reset` to load another.
 */
export const useYamlDraft = (options: IYamlDraftOptions): IYamlDraft => {
    const [controller] = useState(() => new YamlDraftController(options))
    const {localValidate, renderPreview, debounceMs} = options
    useEffect(() => {
        controller.configure({localValidate, renderPreview, debounceMs})
    }, [controller, localValidate, renderPreview, debounceMs])
    useEffect(() => () => controller.dispose(), [controller])
    const state = useSyncExternalStore(controller.subscribe, controller.getState)
    return {...state, controller}
}
