import React, {useCallback, useState} from "react"
import {Sequent_Backend_Candidate} from "@/gql/graphql"
import {DndProvider} from "react-dnd"
import {HTML5Backend} from "react-dnd-html5-backend"
import Candidate from "./Candidate"

const titi = [
    {
        alias: null,
        annotations: null,
        contest_id: "45f037e0-a137-49ee-a053-b4de32d1e5ce",
        created_at: "2024-01-15T13:08:09.518341+00:00",
        description: null,
        election_event_id: "de2904ea-409d-4e65-9da7-264c03539ec3",
        id: "e437aff2-f429-471d-a368-efd11445a792",
        image_document_id: null,
        is_public: false,
        labels: null,
        last_updated_at: "2024-01-15T13:08:09.518341+00:00",
        name: "Ben Reilly",
        presentation: null,
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        type: null,
    },
    {
        alias: null,
        annotations: null,
        contest_id: "45f037e0-a137-49ee-a053-b4de32d1e5ce",
        created_at: "2024-01-15T13:06:13.183398+00:00",
        description: null,
        election_event_id: "de2904ea-409d-4e65-9da7-264c03539ec3",
        id: "ac3b6d49-27a9-4349-a872-1ae02ac0125e",
        image_document_id: null,
        is_public: false,
        labels: null,
        last_updated_at: "2024-01-15T13:06:13.183398+00:00",
        name: "Miles Morales",
        presentation: null,
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        type: null,
    },
    {
        alias: null,
        annotations: null,
        contest_id: "45f037e0-a137-49ee-a053-b4de32d1e5ce",
        created_at: "2024-01-15T13:07:57.833357+00:00",
        description: null,
        election_event_id: "de2904ea-409d-4e65-9da7-264c03539ec3",
        id: "85ca68b1-79cb-45b6-a1d4-bf70d4604ec5",
        image_document_id: null,
        is_public: false,
        labels: null,
        last_updated_at: "2024-01-15T13:07:57.833357+00:00",
        name: "Gwen Stacy",
        presentation: null,
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        type: null,
    },
    {
        alias: null,
        annotations: null,
        contest_id: "45f037e0-a137-49ee-a053-b4de32d1e5ce",
        created_at: "2024-01-15T13:08:20.873461+00:00",
        description: null,
        election_event_id: "de2904ea-409d-4e65-9da7-264c03539ec3",
        id: "6328740e-cf1e-459c-ab74-3f563967eafe",
        image_document_id: null,
        is_public: false,
        labels: null,
        last_updated_at: "2024-01-15T13:08:20.873461+00:00",
        name: "Miguel O'Hara",
        presentation: null,
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        type: null,
    },
    {
        alias: null,
        annotations: null,
        contest_id: "45f037e0-a137-49ee-a053-b4de32d1e5ce",
        created_at: "2024-01-15T13:06:06.595619+00:00",
        description: null,
        election_event_id: "de2904ea-409d-4e65-9da7-264c03539ec3",
        id: "47c0e6c7-a387-4a3d-b2b3-fd98fa95a4e4",
        image_document_id: null,
        is_public: false,
        labels: null,
        last_updated_at: "2024-01-15T13:06:06.595619+00:00",
        name: "Peter Parker",
        presentation: null,
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        type: null,
    },
]

export default function Candidates({list}: {list: Array<Sequent_Backend_Candidate>}) {
    const [candidates, setCandidates] = useState<Array<Sequent_Backend_Candidate>>(titi)

    const moveCard = (dragIndex: number, hoverIndex: number) => {
        console.log("toto dragIndex :>> ", dragIndex)
        console.log("toto hoverIndex :>> ", hoverIndex)

        // setCards((prevCards: Item[]) =>
        //     update(prevCards, {
        //         $splice: [
        //             [dragIndex, 1],
        //             [hoverIndex, 0, prevCards[dragIndex] as Item],
        //         ],
        //     })
        // )

        setCandidates((prevState) => {
            const newState = [...(prevState ?? [])]

            newState.splice(dragIndex, 1)
            newState.splice(hoverIndex, 0, newState[dragIndex] as Sequent_Backend_Candidate)

            return newState
        })
    }

    return (
        <>
            <DndProvider backend={HTML5Backend}>
                <div>
                    {candidates?.map((candidate: any, index: number) => {
                        return (
                            <Candidate
                                key={candidate.id}
                                index={index}
                                id={candidate.id}
                                candidate={candidate}
                                moveCard={moveCard}
                            />
                        )
                    })}
                </div>
            </DndProvider>
        </>
    )
}
