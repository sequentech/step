// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Sequent_Backend_Contest} from "@/gql/graphql"
import {useInput} from "react-admin"
import {Box} from "@mui/material"
import DraggableElement from "@/components/DraggableElement"

export interface OrderContestsProps {
    source: string
}

export const OrderContests: React.FC<OrderContestsProps> = ({source}) => {
    const {
        field: {onChange, value},
    } = useInput({source})

    const [contests, setContests] = useState<Array<Sequent_Backend_Contest>>(value ?? [])
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

        const reorderedItems = [...contests]
        const [reorderedItem] = reorderedItems.splice(dragIndex, 1)
        reorderedItems.splice(dropIndex, 0, reorderedItem)

        setContests(reorderedItems)
        onChange(reorderedItems) // update the form value

        onDragEnd()
    }

    return (
        <Box>
            {contests?.map((contest: Sequent_Backend_Contest, index: number) => {
                return (
                    contest && (
                        <DraggableElement
                            key={contest.id}
                            index={index}
                            id={contest.id}
                            name={contest.name ?? ""}
                            onDragStart={onDragStart}
                            onDragOver={onDragOver}
                            onDrop={onDrop}
                            isOver={overIndex === index}
                        />
                    )
                )
            })}
        </Box>
    )
}
