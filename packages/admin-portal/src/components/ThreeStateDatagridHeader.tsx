// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback} from "react"
import {DatagridHeader, DatagridHeaderProps, SortPayload, useListContext} from "react-admin"
import {DEFAULT_LIST_SORT, resolveThreeStateSort} from "./threeStateSort"

export interface ThreeStateDatagridHeaderProps extends DatagridHeaderProps {
    defaultSort?: SortPayload
}

export const ThreeStateDatagridHeader: React.FC<ThreeStateDatagridHeaderProps> = ({
    defaultSort = DEFAULT_LIST_SORT,
    ...props
}) => {
    const {sort, setSort} = useListContext()
    const handleSetSort = useCallback(
        (requestedSort: SortPayload) => {
            setSort(resolveThreeStateSort(sort, requestedSort, defaultSort))
        },
        [defaultSort, setSort, sort]
    )

    return <DatagridHeader {...props} sort={sort} setSort={handleSetSort} />
}
