// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useListContext} from "react-admin"
import {Sequent_Backend_Candidate} from "../gql/graphql"
import {Link} from "react-router-dom"
import {StyledChip} from "./StyledChip"

export const CandidateChipList: React.FC = () => {
    const {data} = useListContext<Sequent_Backend_Candidate>()
    if (!data) {
        return null
    }

    return (
        <>
            {data.map((candidate) => (
                <Link to={`/sequent_backend_candidate/${candidate.id}`} key={candidate.id}>
                    <StyledChip label={candidate.name} />
                </Link>
            ))}
        </>
    )
}
