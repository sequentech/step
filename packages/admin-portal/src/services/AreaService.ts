// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {create_tree_js, get_contest_matches_js} from "sequent-core"

export interface ITreeNodeArea {
    id: string // area id
    tenant_id: string
    election_event_id: string
    parent_id?: string
}

export interface IContestsData {
    contest_ids: Array<string>
}

export interface TreeNode<T> {
    area?: ITreeNodeArea
    children: Array<TreeNode<T>>
    data: T
}

export interface IAreaContest {
    id: string
    area_id: string
    contest_id: string
}

export const createTree = (
    areas: Array<ITreeNodeArea>,
    areaContests: Array<IAreaContest>
): TreeNode<IContestsData> => {
    try {
        return create_tree_js(areas, areaContests)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getContestMatches = (
    tree: TreeNode<IContestsData>,
    contestId: string
): Array<IAreaContest> => {
    try {
        return get_contest_matches_js(tree, contestId)
    } catch (error) {
        console.log(error)
        throw error
    }
}
