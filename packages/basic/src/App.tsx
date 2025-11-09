
import { isString } from "@sequentech/ui-core"
import { WarnBox } from "@sequentech/ui-essentials"
import React from "react"

export const App: React.FC = () => {
    return <div className="app-root">
        <b>Basic test</b>
        <p>
            1 {
                isString(1)? " is a string": " is not a string"
            }
        </p>
        <WarnBox>A warn box</WarnBox>
    </div>
}