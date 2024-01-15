import React, {useRef} from "react"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"
import {TextField} from "react-admin"
import {useDrag, useDrop} from "react-dnd"
import type {Identifier, XYCoord} from "dnd-core"
import {ItemTypes} from "@/components/types"

export interface CandidateRowItemProps {
    id: any
    candidate: any
    index: number
    moveCard: (dragIndex: number, hoverIndex: number) => void
}

interface DragItem {
    index: number
    id: string
    type: string
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
    const {id, candidate, index, moveCard} = props

    const ref = useRef<HTMLDivElement>(null)

    const [{handlerId}, drop] = useDrop<DragItem, void, {handlerId: Identifier | null}>({
        accept: ItemTypes.CARD,

        collect(monitor) {
            return {
                handlerId: monitor.getHandlerId(),
            }
        },

        hover(item: DragItem, monitor) {
            if (!ref.current) {
                return
            }

            const dragIndex = item.index
            const hoverIndex = index

            // Don't replace items with themselves
            if (dragIndex === hoverIndex) {
                return
            }

            // Determine rectangle on screen
            const hoverBoundingRect = ref.current?.getBoundingClientRect()

            // Get vertical middle
            const hoverMiddleY = (hoverBoundingRect.bottom - hoverBoundingRect.top) / 2

            // Determine mouse position
            const clientOffset = monitor.getClientOffset()

            // Get pixels to the top
            const hoverClientY = (clientOffset as XYCoord).y - hoverBoundingRect.top

            // Only perform the move when the mouse has crossed half of the items height
            // When dragging downwards, only move when the cursor is below 50%
            // When dragging upwards, only move when the cursor is above 50%

            // Dragging downwards
            if (dragIndex < hoverIndex && hoverClientY < hoverMiddleY) {
                return
            }

            // Dragging upwards
            if (dragIndex > hoverIndex && hoverClientY > hoverMiddleY) {
                return
            }

            // Time to actually perform the action
            moveCard(dragIndex, hoverIndex)

            // Note: we're mutating the monitor item here!
            // Generally it's better to avoid mutations,
            // but it's good here for the sake of performance
            // to avoid expensive index searches.
            item.index = hoverIndex
        },
    })

    const [{isDragging}, drag] = useDrag({
        type: ItemTypes.CARD,

        item: () => {
            return {id, index}
        },

        collect: (monitor: any) => ({
            isDragging: monitor.isDragging(),
        }),
    })

    const opacity = isDragging ? 0 : 1

    drag(drop(ref))

    return (
        <div ref={ref} style={{opacity}} data-handler-id={handlerId}>
            <CandidateRow>
                <TextField record={candidate} source="name" />
            </CandidateRow>
        </div>
    )
}
