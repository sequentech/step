import {isString} from "@sequentech/ui-essentials"

export const sortFunction = (a: {name?: string | null}, b: {name?: string | null}) => {
    if (isString(a?.name) && isString(b?.name)) {
        return a.name.localeCompare(b.name)
    }
    return 0
}
