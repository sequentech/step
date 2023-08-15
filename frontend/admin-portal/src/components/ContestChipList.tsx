// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import { useListContext } from "react-admin"
import { Sequent_Backend_Contest } from "../gql/graphql"
import { Link } from "react-router-dom"
import { StyledChip } from "./StyledChip"

export const ContestChipList: React.FC = () => {
    const {data} = useListContext<Sequent_Backend_Contest>()
    if (!data) {
        return null
    }

    return (
        <>
            {data.map((contest) => (
                <Link to={`/sequent_backend_contest/${contest.id}`} key={contest.id}>
                    <StyledChip label={contest.name} />
                </Link>
            ))}
        </>
    )
}