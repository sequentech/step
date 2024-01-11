export const sortFunction = (a: any, b: any) => {
    if (a && a.name && b && b.name) {
        return (a.name as string).localeCompare(b.name as string)
    }
    return 0
}
