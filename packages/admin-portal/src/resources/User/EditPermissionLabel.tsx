// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {useQuery} from "@apollo/client"
import {InputLabel, MenuItem, Select, SxProps} from "@mui/material"
import React, {useCallback} from "react"
import {Trustee} from "./EditUserForm"
import {Identifier} from "react-admin"
import {GET_ELECTION_UNIQUE_PERMISSION_LABLES} from "@/queries/GetElectionPermissionLabels"
interface EditPermissionLabelsProps {
    tenantId: string | null
    id: Identifier | undefined
}
const EditPermissionLabels: React.FC<EditPermissionLabelsProps> = ({tenantId, id}) => {
    const {data: trustees} = useQuery(GET_ELECTION_UNIQUE_PERMISSION_LABLES, {
        variables: {
            tenantId: tenantId,
        },
    })

    return <></>
}

export default EditPermissionLabels
