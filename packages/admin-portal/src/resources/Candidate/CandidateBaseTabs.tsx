import React from "react"
import {ShowBase} from "react-admin"
import {CandidateTabs} from "./CandidateTabs"

export const CandidateBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <CandidateTabs />
        </ShowBase>
    )
}
