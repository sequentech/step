import {IContest} from "@sequentech/ui-essentials"

export interface IContestWithIndex extends IContest {
    originalIndex: number
}

export function sortContestByCreationDate(contests: IContest[]): IContestWithIndex[] {
    // contests = cloneDeep(contests)
    //
    // contests.sort((a, b) => {
    //     const dateA = a.created_at ? new Date(a.created_at) : new Date(0)
    //     const dateB = b.created_at ? new Date(b.created_at) : new Date(0)
    //
    //     return dateA.getTime() - dateB.getTime()
    // })
    //
    // return contests

    // Add a new field 'originalIndex' to each contest
    const contestsWithIndex = contests.map((contest, index) => ({
        ...contest,
        originalIndex: index,
    }))

    // Sort the array with the added 'originalIndex' field
    contestsWithIndex.sort((a, b) => {
        const dateA = a.created_at ? new Date(a.created_at) : new Date(0)
        const dateB = b.created_at ? new Date(b.created_at) : new Date(0)

        return dateA.getTime() - dateB.getTime()
    })

    // Return the sorted array that now includes the 'originalIndex'
    return contestsWithIndex
}
