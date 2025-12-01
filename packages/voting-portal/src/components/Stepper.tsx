// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {BreadCrumbSteps} from "@sequentech/ui-essentials"
import {selectBypassChooser} from "../store/extra/extraSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"

export default function Stepper({selected, warning}: {selected: number; warning?: boolean}) {
    const dispatch = useAppDispatch()
    const bypassElectionChooser = useAppSelector(selectBypassChooser())

    const computedSelected = bypassElectionChooser ? (selected === 0 ? 0 : selected - 1) : selected
    const list = [
        "breadcrumbSteps.ballot",
        "breadcrumbSteps.review",
        "breadcrumbSteps.confirmation",
    ]

    if (!bypassElectionChooser) {
        list.unshift("breadcrumbSteps.electionList")
    }

    return <BreadCrumbSteps labels={list} selected={computedSelected} warning={warning} />
}
