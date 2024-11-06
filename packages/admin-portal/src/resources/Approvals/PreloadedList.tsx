// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren, useState, useEffect, useRef} from "react"
import {useListContext} from "react-admin"

// Define the filter shape
export type FilterValue = string | number | boolean | null | undefined
export type FilterValues = {
    [key: string]: FilterValue | FilterValue[] | {[key: string]: FilterValue}
}

interface PreloadedListProps extends PropsWithChildren {
    defaultFilters: FilterValues | undefined
    resource?: string
    [key: string]: any // for any additional props that might be passed
}

export const PreloadedList: React.FC<PreloadedListProps> = ({
    defaultFilters,
    children,
    resource,
    ...props
}) => {
    const {setFilters, filterValues} = useListContext<any>()
    const hasSetInitialFilters = useRef<boolean>(false)

    useEffect(() => {
        console.log("bb Current filters:", filterValues)
        console.log("bb Default filters:", defaultFilters)
        console.log("bb Has set initial:", hasSetInitialFilters.current)

        // Only set filters once when defaultFilters are available and not yet set
        if (
            defaultFilters &&
            !hasSetInitialFilters.current &&
            Object.keys(defaultFilters).length > 0
        ) {
            console.log("bb PASO POR SETFILTERS ===== ======")
            setFilters(defaultFilters, {})
            hasSetInitialFilters.current = true
        }
    }, [defaultFilters, setFilters])

    // Prevent filter reset on re-renders
    useEffect(() => {
        return () => {
            hasSetInitialFilters.current = false
        }
    }, [])

    return <>{children}</>
}
