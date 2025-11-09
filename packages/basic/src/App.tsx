
import { isString } from "@sequentech/ui-core"
import { MyBox } from "@sequentech/ui-essentials"
import React from "react"

export const App: React.FC = () => {
    return <div className="app-root">
        <b>Basic test</b>
        <p>
            1 {
                isString(1)? " is a string": " is not a string"
            }
        </p>
        <MyBox></MyBox>
    </div>
}