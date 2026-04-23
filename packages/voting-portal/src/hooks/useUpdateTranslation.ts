// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {IElectionEvent} from "../store/electionEvents/electionEventsSlice"
import {overwriteTranslations} from "@sequentech/ui-core"

type props = {
    electionEvent: IElectionEvent | undefined
}
const useUpdateTranslation = (
    {electionEvent}: props,
    defaultLanguageTouched: boolean,
    setDefaultLanguageTouched: (value: boolean) => void
) => {
    const [, setTranslationRefreshTick] = useState(0)

    // Overwrites translations based on the election event presentation
    useEffect(() => {
        if (!electionEvent?.presentation) {
            return
        }
        let hasSetDefaultLanguage = overwriteTranslations(
            electionEvent?.presentation,
            !defaultLanguageTouched
        )
        if (hasSetDefaultLanguage) {
            setDefaultLanguageTouched(true)
        }

        // Force one rerender after adding resource bundles so components using
        // translated keys in render pick up overwritten values immediately.
        setTranslationRefreshTick((tick) => tick + 1)
    }, [electionEvent?.presentation])

    return {}
}

export default useUpdateTranslation
