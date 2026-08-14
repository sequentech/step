// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {SortPayload} from "react-admin"

export const DEFAULT_LIST_SORT: SortPayload = {field: "id", order: "ASC"}

export const resolveThreeStateSort = (
    currentSort: SortPayload | undefined,
    requestedSort: SortPayload,
    defaultSort: SortPayload = DEFAULT_LIST_SORT
): SortPayload =>
    currentSort?.field === requestedSort.field &&
    currentSort.order === "DESC" &&
    requestedSort.order === "ASC"
        ? defaultSort
        : requestedSort
