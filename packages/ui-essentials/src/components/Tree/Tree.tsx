// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
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
    gap: 8px;
    align-items: center;
    cursor: pointer;
`

const LeavesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    margin-left: 24px;
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
            <Horizontal onClick={onClick}>
                {leaves ? <Icon icon={open ? faAngleDown : faAngleRight} /> : null}
                <Typography>{label}</Typography>
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
    parent: TreeLeaveProps
    props?: BoxProps
}

const Tree: React.FC<TreeProps> = ({parent, props}) => {
    return (
        <Box {...props}>
            <TreeLeave {...parent} />
        </Box>
    )
}

export default Tree
