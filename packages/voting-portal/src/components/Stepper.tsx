import React, {useEffect} from "react"
import {BreadCrumbSteps} from "@sequentech/ui-essentials"
import {useBypassElectionChooser} from "../hooks/bypass-election-chooser"

export default function Stepper({selected, warning}: {selected: number; warning?: boolean}) {
    const bypassElectionChooser = useBypassElectionChooser()

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
