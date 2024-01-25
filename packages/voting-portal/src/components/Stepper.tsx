import React, {useEffect} from "react"
import {BreadCrumbSteps} from "@sequentech/ui-essentials"

export default function Stepper({selected}: {selected: number}) {
    return (
        <BreadCrumbSteps
            labels={[
                "breadcrumbSteps.electionList",
                "breadcrumbSteps.ballot",
                "breadcrumbSteps.review",
                "breadcrumbSteps.confirmation",
            ]}
            selected={selected}
        />
    )
}
