// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {RaRecord, useListContext} from "react-admin"
import {Link} from "react-router-dom"
import {StyledChip} from "./StyledChip"

export interface ChipListProps {
    source: string
    filterFields?: Array<string>
    max?: number
}

const DEFAULT_MAX = 10

export const ChipList: React.FC<ChipListProps> = ({source, filterFields, max}) => {
    const {data} = useListContext<RaRecord>()
    if (!data) {
        return null
    }
    const handleClick: React.MouseEventHandler<HTMLAnchorElement> = (event) => {
        event.stopPropagation()
    }

    const filter = (input: Record<string, any>): Record<string, any> => {
        let o: Record<string, any> = {}
        if (filterFields) {
            for (let key of filterFields) {
                o[key] = input[key]
            }
        }
        return o
    }

    return (
        <>
            {data.slice(0, max || DEFAULT_MAX).map((element) => (
                <Link
                    to={{
                        pathname: `/${source}/${element.id}`,
                        search: filterFields
                            ? `filter=${JSON.stringify(filter(element))}`
                            : undefined,
                    }}
                    key={element.id}
                    onClick={handleClick}
                >
                    <StyledChip label={element.name} />
                </Link>
            ))}
        </>
    )
}
