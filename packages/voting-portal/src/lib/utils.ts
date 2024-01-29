import {IContest} from "@sequentech/ui-essentials"
import {cloneDeep} from "lodash"

export function sortContestByCreationDate(contests: IContest[]): IContest[] {
    contests = cloneDeep(contests)

    contests.sort((a, b) => {
        const dateA = a.created_at ? new Date(a.created_at) : new Date(0)
        const dateB = b.created_at ? new Date(b.created_at) : new Date(0)

        return dateA.getTime() - dateB.getTime()
    })

    return contests
}
