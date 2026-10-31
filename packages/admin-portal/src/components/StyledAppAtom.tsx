// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useAtomValue} from "jotai"
import cssInputLookAndFeel from "@/atoms/css-input-look-and-feel"

const StyledApp = styled(Box, {
    shouldForwardProp: (prop) => prop !== "customCss",
})<{customCss: string}>`
    ${({customCss}) => customCss}
`

/** Applies the tenant's look-and-feel CSS to its children. */
export const StyledAppAtom: React.FC<{children: React.ReactNode}> = ({children}) => {
    const css = useAtomValue(cssInputLookAndFeel)
    return (
        <StyledApp className="styled-app-atom" customCss={css}>
            {children}
        </StyledApp>
    )
}
