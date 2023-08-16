// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {RaRecord, useListContext} from "react-admin"
import {Link} from "react-router-dom"
import {StyledChip} from "./StyledChip"

export interface ChipListProps {
    source: string
}

interface DataRecord extends RaRecord {
    name: string
}

export const ChipList: React.FC<ChipListProps> = ({source}) => {
    const {data} = useListContext<DataRecord>()
    if (!data) {
        return null
    }
    const handleClick: React.MouseEventHandler<HTMLAnchorElement> = (event) => {
        event.stopPropagation()
    }

    return (
        <>
            {data.map((element) => (
                <Link to={`/${source}/${element.id}`} key={element.id} onClick={handleClick}>
                    <StyledChip label={element.name} />
                </Link>
            ))}
        </>
    )
}
