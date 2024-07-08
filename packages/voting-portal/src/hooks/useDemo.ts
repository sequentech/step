import {useState, useEffect, useMemo} from "react"
import {useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

const useDemo = () => {
    const [isDemo, setIsDemo] = useState(false)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)

    useEffect(() => {
        const url = window.location.href
        if (url.includes("demo") || oneBallotStyle?.ballot_eml.public_key?.is_demo) {
            setIsDemo(true)
        }
    }, [])

    const isDemoMemoized = useMemo(() => {
        return isDemo
    }, [isDemo])

    return isDemoMemoized
}

export default useDemo
