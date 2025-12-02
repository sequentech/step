// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactNode, useState} from "react"
import {Box, BoxProps, Typography} from "@mui/material"
import {faAngleRight, faAngleDown} from "@fortawesome/free-solid-svg-icons"
import Icon from "../Icon/Icon"
import {styled} from "@mui/material/styles"

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    cursor: pointer;
`

const LeavesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    margin-left: 24px;
`

const StyledIcon = styled(Icon)`
    width: 24px;
`

export interface TreeLeaveProps {
    label: ReactNode | string
    leaves?: Array<TreeLeaveProps>
    props?: BoxProps
    defaultOpen?: boolean
}

const TreeLeave: React.FC<TreeLeaveProps> = ({props, label, leaves, defaultOpen}) => {
    const [open, setOpen] = useState(defaultOpen || false)

    const onClick = () => setOpen(!open)
    return (
        <Box {...props}>
            <Horizontal>
                {leaves ? (
                    <StyledIcon icon={open ? faAngleDown : faAngleRight} onClick={onClick} />
                ) : null}
                <Typography fontSize="16px" margin={0}>
                    {label}
                </Typography>
            </Horizontal>
            {open ? (
                <LeavesWrapper>
                    {leaves?.map((child, idx) => (
                        <TreeLeave {...child} key={idx} />
                    ))}
                </LeavesWrapper>
            ) : null}
        </Box>
    )
}

export interface TreeProps {
    leaves: Array<TreeLeaveProps>
    props?: BoxProps
}

const Tree: React.FC<TreeProps> = ({leaves, props}) => {
    return (
        <Box {...props}>
            {leaves.map((leave, index) => (
                <TreeLeave {...leave} key={index} />
            ))}
        </Box>
    )
}

export default Tree
