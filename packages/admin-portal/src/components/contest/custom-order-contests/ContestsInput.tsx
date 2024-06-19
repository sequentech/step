// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {Sequent_Backend_Contest} from "@/gql/graphql"
import {useInput} from "react-admin"
import {Box} from "@mui/material"
import DraggableElement from "@/components/DraggableElement"
import {isArray} from "@sequentech/ui-essentials"

export interface ContestsInputProps {
    source: string
}

const ContestsInput: React.FC<ContestsInputProps> = ({source}) => {
    const {
        field: {onChange, value},
    } = useInput({source})

	console.log('contests input', {value, source, onChange})

    const [contests, setContests] = useState<Array<Sequent_Backend_Contest>>(value ?? [])
    const [dragIndex, setDragIndex] = useState<number>(-1)
    const [overIndex, setOverIndex] = useState<number | null>(null)

    useEffect(() => {
        if (isArray(value) && value.length > 0 && value.length !== contests.length) {
            setContests(value)
        }
    }, [value, contests, setContests, isArray])

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
		console.log('reorder items')
        event.preventDefault()

        if (dragIndex === -1 || dragIndex === dropIndex) {
            return
        }

        const reorderedItems = [...contests]
        const [reorderedItem] = reorderedItems.splice(dragIndex, 1)
        reorderedItems.splice(dropIndex, 0, reorderedItem)
		console.log({reorderedItems})

        setContests(reorderedItems)
        onChange(reorderedItems.map((v, i)=>{
			console.log({v})
			return {
				...v,
				presentation: {
					...v.presentation,
					x: i
				}
			}
		})) // update the form value

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

export default ContestsInput
