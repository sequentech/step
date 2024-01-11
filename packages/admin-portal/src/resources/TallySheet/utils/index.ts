import {isString} from "@sequentech/ui-essentials"

export const sortFunction = (a: {name?: string}, b: {name?: string}) => {
    if (isString(a?.name) && isString(b?.name)) {
        return a.name.localeCompare(b.name)
    }
    return 0
}
