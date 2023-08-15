// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useListContext} from "react-admin"
import {Sequent_Backend_Area} from "../gql/graphql"
import {Link} from "react-router-dom"
import {StyledChip} from "./StyledChip"

export const AreaChipList: React.FC = () => {
    const {data} = useListContext<Sequent_Backend_Area>()
    if (!data) {
        return null
    }

    return (
        <>
            {data.map((area) => (
                <Link to={`/sequent_backend_area/${area.id}`} key={area.id}>
                    <StyledChip label={area.name} />
                </Link>
            ))}
        </>
    )
}
