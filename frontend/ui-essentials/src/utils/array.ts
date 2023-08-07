import {shuffle as DashShuffle} from "moderndash"

export const shuffle = DashShuffle

export const splitList = <T>(
    list: Array<T>,
    test: (element: T) => boolean
): [Array<T>, Array<T>] => {
    const negative: Array<T> = []
    const positive: Array<T> = []

    for (let element of list) {
        if (test(element)) {
            positive.push(element)
        } else {
            negative.push(element)
        }
    }

    return [negative, positive]
}
