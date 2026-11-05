// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {RaRecord, useListContext} from "react-admin"
import {Link} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {translateFromPresentation} from "@sequentech/ui-core"
import {StyledChip} from "./StyledChip"
import {stringifyFields} from "../services/RowClickService"

export interface ChipListProps {
    source: string
    filterFields?: Array<string>
    max?: number
}

const DEFAULT_MAX = 10

interface ChipRecord extends RaRecord {
    name?: string | null
    presentation?: {i18n?: Record<string, Record<string, string | null>>} | null
}

export const ChipList: React.FC<ChipListProps> = ({source, filterFields, max}) => {
    const {data} = useListContext<ChipRecord>()
    const {i18n} = useTranslation()
    if (!data) {
        return null
    }
    const handleClick: React.MouseEventHandler<HTMLAnchorElement> = (event) => {
        event.stopPropagation()
    }

    return (
        <>
            {data.slice(0, max || DEFAULT_MAX).map((element) => (
                <Link
                    to={{
                        pathname: `/${source}/${element.id}`,
                        search: filterFields
                            ? `filter=${stringifyFields(element, filterFields)}`
                            : undefined,
                    }}
                    key={element.id}
                    onClick={handleClick}
                >
                    <StyledChip
                        label={translateFromPresentation(element, "name", i18n.language, {
                            defaultLanguageCode: "en",
                        })}
                    />
                </Link>
            ))}
        </>
    )
}
