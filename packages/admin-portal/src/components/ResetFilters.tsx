// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import {useListContext} from "react-admin"

export const ResetFilters: React.FC = () => {
    const {setFilters} = useListContext()

    useEffect(() => {
        const resetFilters = () => {
            // Reset filters when the component mounts
            if (setFilters) {
                setFilters({}, {})
            }
        }
        resetFilters()
    }, [])

    return <></>
}
