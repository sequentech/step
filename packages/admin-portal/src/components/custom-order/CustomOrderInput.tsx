// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box} from "@mui/material"
import {isArray} from "@sequentech/ui-core"
import React, {useEffect, useState} from "react"
import {useInput} from "react-admin"
import DraggableElement from "../DraggableElement"

type CustomOrderInputProps = {
    source: string
}

const CustomOrderInput = ({source}: CustomOrderInputProps) => {
    const {
        field: {onChange, value},
    } = useInput({source})

    const [data, setData] = useState<Array<any>>(value ?? [])
    const [dragIndex, setDragIndex] = useState<number>(-1)
    const [overIndex, setOverIndex] = useState<number | null>(null)

    useEffect(() => {
        if (isArray(value) && value.length > 0) {
            setData(value)
        }
    }, [value, setData, isArray])

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

        setData((prev) => {
            if (!prev || !prev.length) return []
            const reorderedItems = [...prev]
            const [draggedItem] = reorderedItems.splice(dragIndex, 1)

            reorderedItems.splice(dropIndex, 0, draggedItem)

            onChange(reorderedItems) // Update the form value
            return reorderedItems
        })
        onDragEnd()
    }

    return (
        <Box>
            {data?.map((lineItem: any, index: number) => {
                return (
                    lineItem && (
                        <DraggableElement
                            key={lineItem.id}
                            index={index}
                            id={lineItem.id}
                            name={lineItem?.name ?? ""}
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

export default CustomOrderInput
