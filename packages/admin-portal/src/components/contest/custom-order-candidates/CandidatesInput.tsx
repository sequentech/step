import React, {useState} from "react"
import {Sequent_Backend_Candidate} from "@/gql/graphql"
import Candidate from "./Candidate"
import {useInput} from "react-admin"

export default function CandidatesInput({source}: {source: string}) {
    const {
        field: {onChange, value},
    } = useInput({source})

    const [candidates, setCandidates] = useState<Array<Sequent_Backend_Candidate>>(value ?? [])
    const [dragIndex, setDragIndex] = useState<number>(-1)
    const [overIndex, setOverIndex] = useState<number | null>(null)

    const onDragStart = (_event: React.DragEvent<HTMLDivElement>, index: number) => {
        setDragIndex(index)
    }

    const onDragOver = (event: React.DragEvent<HTMLDivElement>, index: number) => {
        event.preventDefault()

        setOverIndex(index)
    }

    const onDragEnd = () => {
        setOverIndex(-1)
        setDragIndex(-1)
    }

    const onDrop = (event: React.DragEvent<HTMLDivElement>, dropIndex: number) => {
        event.preventDefault()

        if (dragIndex === -1 || dragIndex === dropIndex) {
            return
        }

        const reorderedItems = [...candidates]
        const [reorderedItem] = reorderedItems.splice(dragIndex, 1)
        reorderedItems.splice(dropIndex, 0, reorderedItem)

        setCandidates(reorderedItems)
        onChange(reorderedItems.map((c) => c?.id ?? null).filter((c) => !!c)) // update the form value

        onDragEnd()
    }

    return (
        <>
            <div>
                {candidates?.map((candidate: any, index: number) => {
                    return (
                        candidate && (
                            <Candidate
                                key={candidate.id}
                                index={index}
                                id={candidate.id}
                                candidate={candidate}
                                onDragStart={onDragStart}
                                onDragOver={onDragOver}
                                onDrop={onDrop}
                                isOver={overIndex === index}
                            />
                        )
                    )
                })}
            </div>
        </>
    )
}
