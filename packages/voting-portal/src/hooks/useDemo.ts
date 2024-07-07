import {useState, useEffect, useMemo} from "react"

const useDemo = () => {
    const [isDemo, setIsDemo] = useState(false)

    useEffect(() => {
        const url = window.location.href
        if (url.includes("demo")) {
            setIsDemo(true)
        }
    }, [])

    const isDemoMemoized = useMemo(() => {
        return isDemo
    }, [isDemo])

    return isDemoMemoized
}

export default useDemo
