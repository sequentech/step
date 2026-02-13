// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {useQuery} from "@apollo/client"
import {InputLabel, MenuItem, Select} from "@mui/material"
import React, {useEffect, useState} from "react"
import {Trustee} from "./EditUserForm"
interface SelectActedTrusteeProps {
    tenantId: string | null
    onSelectTrustee: (trustee: string) => void
    defaultValue?: string | string[]
    source: string
    label?: string
}
const SelectActedTrustee: React.FC<SelectActedTrusteeProps> = ({
    tenantId,
    onSelectTrustee,
    defaultValue,
    label,
}) => {
    const [value, setValue] = useState("")

    const {data: trustees} = useQuery(GET_TRUSTEES_NAMES, {
        variables: {
            tenantId: tenantId,
        },
    })

    useEffect(() => {
        if (defaultValue) {
            const trustee = trustees?.sequent_backend_trustee.find(
                (trustee: {id: string; name: string}) => trustee.name === defaultValue
            )
            handleChangeTrustee?.(trustee?.name ?? "")
        }
    }, [trustees, defaultValue])

    const handleChangeTrustee = (v: string) => {
        onSelectTrustee(v)
        setValue(v)
    }

    return (
        <>
            {label && <InputLabel id="select-label">{label}</InputLabel>}
            <Select
                name={"Acted trustee"}
                labelId="trustee"
                label={label}
                value={value}
                onChange={(e) => handleChangeTrustee(e.target.value)}
            >
                <MenuItem key={"empty-value"} value={" "}>
                    {" "}
                </MenuItem>
                {trustees?.sequent_backend_trustee?.map((trustee: Trustee) => {
                    return (
                        <MenuItem key={trustee.id} value={trustee.name}>
                            {trustee.name}
                        </MenuItem>
                    )
                })}
            </Select>
        </>
    )
}

export default SelectActedTrustee
