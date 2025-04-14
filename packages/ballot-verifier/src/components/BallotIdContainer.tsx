// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import Paper, {PaperProps} from "@mui/material/Paper"
import {styled} from "@mui/material/styles"

const BallotIdPaper = styled(Paper)`
    padding: 10px 16px;
    display: flex;
    overflow: auto;
`

enum VariantType {
    Info = "info",
    Error = "error",
}

interface BallotIdContainerProps extends PaperProps {
    variant: VariantType
}

export const BallotIdContainer: React.FC<PropsWithChildren<BallotIdContainerProps>> = ({
    variant,
    children,
    ...props
}) => (
    <BallotIdPaper variant={variant} {...props}>
        {children}
    </BallotIdPaper>
)
