import {IContest} from "@sequentech/ui-essentials"

export interface IContestWithIndex extends IContest {
    originalIndex: number
}

export function sortContestByCreationDate(contests: IContest[]): IContestWithIndex[] {
    const contestsWithIndex = contests.map((contest, index) => ({
        ...contest,
        originalIndex: index,
    }))

    contestsWithIndex.sort((a, b) => {
        const dateA = a.created_at ? new Date(a.created_at) : new Date(0)
        const dateB = b.created_at ? new Date(b.created_at) : new Date(0)

        return dateA.getTime() - dateB.getTime()
    })

    return contestsWithIndex
}
