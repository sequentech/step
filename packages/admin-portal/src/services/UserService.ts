export const getAttributeLabel = (displayName: string) => {
    if (displayName?.includes("$")) {
        return (
            displayName
                .replace(/^\${|}$/g, "")
                .trim()
                .replace(/([a-z])([A-Z])/g, "$1 $2")
                .replace(/^./, (match) => match.toUpperCase()) ?? ""
        )
    }
    return displayName ?? ""
}

export const userBasicInfo = ["first_name", "last_name", "email", "username"]
