// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box} from "@mui/material"

import styled from "@emotion/styled"
import Stats from "./Stats"
import VotesByDay from "../charts/VoteByDay"
import VotesByChannel from "../charts/VoteByChannels"
import {useRecordContext} from "react-admin"
import {Sequent_Backend_Election} from "@/gql/graphql"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElection() {
    const cardWidth = 470
    const cardHeight = 250

    const record = useRecordContext<Sequent_Backend_Election>()

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <Box>
                    <Stats electionEventId={record.election_event_id} electionId={record.id} />

                    <Container>
                        <VotesByDay width={cardWidth} height={cardHeight} />
                        <VotesByChannel
                            electionEventId={record.election_event_id}
                            electionId={record.id}
                            width={cardWidth}
                            height={cardHeight}
                        />
                    </Container>
                </Box>
            </Box>
        </>
    )
}
