// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { tallyAreas } from "@/atoms/tally-candidates"
import { GetTallyDataQuery, Sequent_Backend_Area, Sequent_Backend_Area_Contest } from "@/gql/graphql"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { GET_TALLY_DATA } from "@/queries/GetTallyData"
import { useQuery } from "@apollo/client"
import { isUndefined } from "@sequentech/ui-essentials"
import { useSetAtom } from "jotai"
import React, {useContext, useEffect, useMemo, useState} from "react"
import { useGetList } from "react-admin"

export interface ResultsDataLoaderProps {
    resultsEventId: string
    electionEventId: string
}

export const ResultsDataLoader: React.FC<ResultsDataLoaderProps> = ({resultsEventId, electionEventId}) => {
    const [tenantId] = useTenantStore()
    const setAreasData = useSetAtom(tallyAreas)

    const {data: tallyData} = useQuery<GetTallyDataQuery>(GET_TALLY_DATA, {
        variables: {
            resultsEventId,
            electionEventId,
            tenantId,
        },
    })

    return <></>
}