// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useEffect, useRef, useState} from "react"
import {Box} from "@mui/material"
import {sortBy} from "lodash"
import styled from "@emotion/styled"

const ContestsWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
`

const ContestContainer = styled(Box)`
    display: flex;
    flex-direction: column;
`

interface ContestData {
    id: string
    order: number
}

const Contest: React.FC<ContestData> = ({id, order}) => {
    const ref = useRef<HTMLDivElement>(null)

    return (
        <ContestContainer ref={ref} draggable>
            {id}
        </ContestContainer>
    )
}

export const OrderContests: React.FC = () => {
    const [contests, setContests] = useState<Array<ContestData>>([
        {
            id: "A",
            order: 0,
        },
        {
            id: "B",
            order: 1,
        },
        {
            id: "C",
            order: 2,
        },
    ])

    const orderedContests = sortBy(contests, "order")

    return (
        <Box>
            Order these contests:
            <ContestsWrapper>
                {orderedContests.map((contest) => (
                    <Contest key={contest.id} id={contest.id} order={contest.order} />
                ))}
            </ContestsWrapper>
        </Box>
    )
}
