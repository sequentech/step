// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import { useListContext } from "react-admin"
import { Sequent_Backend_Election } from "../gql/graphql"
import { Link } from "react-router-dom"
import { StyledChip } from "./StyledChip"

export const ElectionChipList: React.FC = () => {
    const {data} = useListContext<Sequent_Backend_Election>()
    if (!data) {
        return null
    }

    return (
        <>
            {data.map((election) => (
                <Link to={`/sequent_backend_election/${election.id}`} key={election.id}>
                    <StyledChip label={election.name} />
                </Link>
            ))}
        </>
    )
}