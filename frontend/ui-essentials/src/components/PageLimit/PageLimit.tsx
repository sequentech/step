// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Breakpoint, styled} from "@mui/material/styles"
import Container, {ContainerProps} from "@mui/material/Container"

const CustomContainer = styled(Container)`
    padding: 0px 8px !important;
`

type PageLimitProps = ContainerProps & {maxWidth?: Breakpoint}

const PageLimit: React.FC<PageLimitProps> = ({children, maxWidth, ...props}) => (
    <CustomContainer maxWidth={maxWidth || "sm"} {...props}>
        {children}
    </CustomContainer>
)

export default PageLimit
