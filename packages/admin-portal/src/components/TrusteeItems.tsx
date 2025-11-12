// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {styled} from "@mui/material/styles"
import React from "react"
import {Identifier, RaRecord, useGetOne} from "react-admin"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define the actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

interface TrusteeItemsProps {
    record: RaRecord<Identifier>
    trusteeNames?: Array<{
        id?: string
        name?: string | null
    }>
}

const StyledChips = styled("div")`
    display: flex;
    padding: 1px 7px;
    flex-direction: row;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 4px;
`

const StyledChip = styled("div")`
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 14px;
    background: #ebebeb;
    padding: 7px;
`

const StyledChipLabel = styled("div")`
    font-family: Roboto;
    font-size: 12px;
    font-style: normal;
    font-weight: 400;
    line-height: 18px;
`

export const TrusteeItems: React.FC<TrusteeItemsProps> = (props) => {
    const {record, trusteeNames} = props

    const {data: keyCeremony} = useGetOne("sequent_backend_keys_ceremony", {
        id: record.keys_ceremony_id,
    })

    let filteredTrustees =
        trusteeNames?.filter((trustee) => keyCeremony?.trustee_ids?.includes(trustee.id)) ?? []

    return (
        <StyledChips>
            {filteredTrustees?.map((item, index: number) => (
                <StyledChip key={index}>
                    <StyledChipLabel>{item.name}</StyledChipLabel>
                </StyledChip>
            ))}
        </StyledChips>
    )
}
