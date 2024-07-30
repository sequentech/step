// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {IElectionEvent} from "../store/electionEvents/electionEventsSlice"
import {overwriteTranslations} from "@sequentech/ui-core"

type props = {
    electionEvent: IElectionEvent | undefined
}
const useUpdateTranslation = ({electionEvent}: props) => {
    // Overwrites translations based on the election event presentation
    useEffect(() => {
        if (!electionEvent?.presentation) return
        overwriteTranslations(electionEvent)
    }, [electionEvent])

    return {}
}

export default useUpdateTranslation
