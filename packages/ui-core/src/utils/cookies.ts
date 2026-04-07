export const getValueFromCookie = (cookieName: string) => {
    const cookies = Object.fromEntries(document.cookie.split("; ").map((c) => c.split("=")))
    const value = cookies[cookieName]

    return value || undefined
}
