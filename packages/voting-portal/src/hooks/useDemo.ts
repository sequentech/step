// SPDX-FileCopyrightText: 2024 Sequent Tech <leagal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useState, useEffect} from "react"
import {useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

const useDemo = () => {
    const [isDemo, setIsDemo] = useState(false)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)

    useEffect(() => {
        const url = window.location.search
        if (url.includes("demo") || oneBallotStyle?.ballot_eml.public_key?.is_demo) {
            setIsDemo(true)
        }
    }, [oneBallotStyle?.ballot_eml.public_key?.is_demo])

    return isDemo
}

export default useDemo
