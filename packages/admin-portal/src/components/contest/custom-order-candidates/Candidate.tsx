// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import styled from "@emotion/styled"
import {theme, ICandidate} from "@sequentech/ui-essentials"
import {TextField} from "react-admin"
import DragIndicatorIcon from "@mui/icons-material/DragIndicator"

export interface CandidateRowItemProps {
    id: any
    candidate: ICandidate
    index: number
    onDragStart: (event: React.DragEvent<HTMLDivElement>, index: number) => void
    onDragOver: (event: React.DragEvent<HTMLDivElement>, index: number) => void
    onDrop: (event: React.DragEvent<HTMLDivElement>, index: number) => void
    isOver: boolean
}

const CandidateRow = styled.div`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    cursor: move;
    margin-bottom: 0.1rem;
    padding: 0.3rem 1rem;
    border-radius: 1rem;
    border: 2px dashed ${theme.palette.grey[500]};
    transition: background-color 0.2s ease-in-out, box-shadow 0.2s ease-in-out; // Smooth transition for background color and shadow

    &:hover {
        border: 2px dashed ${theme.palette.primary.main};
    }

    &.dragging {
        opacity: 0.5;
        box-shadow: 0 5px 10px rgba(0, 0, 0, 0.3);
    }

    &.over {
        border: 2px dashed ${theme.palette.secondary.main};
    }
`

export default function Candidate(props: CandidateRowItemProps) {
    const {candidate, index, onDragStart, onDragOver, onDrop, isOver} = props
    const [isDragging, setIsDragging] = useState(false)

    const ref = useRef<HTMLDivElement>(null)

    const handleDragStart = (event: React.DragEvent<HTMLDivElement>, index: number) => {
        setIsDragging(true)
        onDragStart(event, index)
    }

    const handleDragEnd = (_event: React.DragEvent<HTMLDivElement>) => {
        setIsDragging(false)
    }

    return (
        <div
            ref={ref}
            draggable
            onDragStart={(event) => handleDragStart(event, index)}
            onDragOver={(event) => onDragOver(event, index)}
            onDrop={(event) => onDrop(event, index)}
            onDragEnd={handleDragEnd}
        >
            <CandidateRow className={isDragging ? "dragging" : isOver ? "over" : ""}>
                <DragIndicatorIcon sx={{mr: 1}} />
                <TextField record={candidate} source="name" />
            </CandidateRow>
        </div>
    )
}
