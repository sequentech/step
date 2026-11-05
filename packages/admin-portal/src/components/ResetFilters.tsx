// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import {useListContext} from "react-admin"
import {useLocation} from "react-router-dom"

export interface IResetFiltersProps {
    /**
     * Keep the filter the location carries, so a link that opens the list
     * already filtered (such as Failed → Logs) shows what it promised. The
     * filter shows and can be cleared like any other.
     */
    keepLinkedFilter?: boolean
}

/** Clears the list's filters when it opens. */
export const ResetFilters: React.FC<IResetFiltersProps> = ({keepLinkedFilter = false}) => {
    const {setFilters} = useListContext()
    const location = useLocation()

    useEffect(() => {
        const linked = new URLSearchParams(location.search).has("filter")
        if (setFilters && !(keepLinkedFilter && linked)) {
            setFilters({}, {})
        }
    }, [])

    return <></>
}
