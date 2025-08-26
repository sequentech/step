// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
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
    }, [electionEvent?.presentation])

    return {}
}

export default useUpdateTranslation
