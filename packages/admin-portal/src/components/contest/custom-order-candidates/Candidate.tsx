import React, {useRef} from "react"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"
import {TextField} from "react-admin"

export interface CandidateRowItemProps {
    id: any
    candidate: any
    index: number
    onDragStart: (event: React.DragEvent<HTMLDivElement>, index: number) => void
    onDragOver: (event: React.DragEvent<HTMLDivElement>) => void
    onDrop: (event: React.DragEvent<HTMLDivElement>, index: number) => void
}

const CandidateRow = styled.div`
    display: flex;
    flex-direction: column;
    width: 100%;
    cursor: move;
    margin-bottom: 0.1rem;
    padding: 0.3rem 1rem;
    border-radius: 1rem;
    border: 2px dashed ${theme.palette.grey[500]};
    &:hover {
        background-color: ${theme.palette.lightBackground};
      }
    }
`

export default function Candidate(props: CandidateRowItemProps) {
    const {id, candidate, index, onDragStart, onDragOver, onDrop} = props

    const ref = useRef<HTMLDivElement>(null)

    return (
        <div
            ref={ref}
            draggable
            onDragStart={(event) => onDragStart(event, index)}
            onDragOver={onDragOver}
            onDrop={(event) => onDrop(event, index)}
        >
            <CandidateRow>
                <TextField record={candidate} source="name" />
            </CandidateRow>
        </div>
    )
}
