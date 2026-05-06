// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Collapse, Typography} from "@mui/material"
import React, {PropsWithChildren, useState} from "react"
import {styled} from "@mui/material/styles"
import theme from "../../services/theme"
import {Checkbox} from "@mui/material"
import {faAngleDown, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import IconButton from "../IconButton/IconButton"

const ListContainer = styled(Box)<{isactive: string}>`
    backgroundcolor: ${({theme}) => theme.palette.lightBackground};
    padding: 0 14px 20px 16px;
    box-shadow: 0 2px 4px 2px rgba(0, 0, 0, 0.25);
    border-radius: 5px;
    flex-grow: 2;
    width: 50%;
    ${({isactive}) =>
        "true" === isactive
            ? `
            &:hover {
                cursor: pointer;
            }
        `
            : ""}
`

const ListHeader = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
`

const ListChildrenContainer = styled("ul")`
    flex-grow: 2;
    list-style: none;
    margin: 12px 0;
    padding-inline-start: 0;
    gap: 40px;
    flex-wrap: wrap;
    li + li {
        margin-top: 12px;
    }
`

const ListTitle = styled(Typography)`
    margin-top: 10px;
    margin-bottom: 26px;
    flex-shrink: 0;
    flex-grow: 2;
    text-align: center;
    font-size: 24px;
`

export interface CandidatesListProps extends PropsWithChildren {
    title: string
    isActive?: boolean
    isCheckable?: boolean
    checked?: boolean
    setChecked?: (value: boolean) => void
    isCollapsible?: boolean
    defaultExpanded?: boolean
    collapseToggleAriaLabel?: string
}

const CandidatesList: React.FC<CandidatesListProps> = ({
    title,
    children,
    isActive,
    isCheckable,
    checked,
    setChecked,
    isCollapsible,
    defaultExpanded,
    collapseToggleAriaLabel,
}) => {
    const [isExpanded, setIsExpanded] = useState<boolean>(defaultExpanded ?? true)

    const onClick = () => {
        if (isActive && isCheckable && setChecked) {
            setChecked(!checked)
        }
    }
    const handleChange = (event: React.ChangeEvent<HTMLInputElement>) =>
        isActive && isCheckable && setChecked && setChecked(event.target.checked)

    const handleToggleCollapse = (event: React.MouseEvent) => {
        event.stopPropagation()
        setIsExpanded((prev) => !prev)
    }

    return (
        <ListContainer
            isactive={String(!!(isActive && isCheckable))}
            onClick={onClick}
            className="candidates-list"
        >
            <ListHeader className="candidates-list-header">
                {isCollapsible ? (
                    <IconButton
                        icon={isExpanded ? faAngleDown : faAngleRight}
                        onClick={handleToggleCollapse}
                        title={collapseToggleAriaLabel}
                    />
                ) : null}
                <Box sx={{flexGrow: 2}}>
                    <ListTitle
                        color={theme.palette.customGrey.contrastText}
                        fontSize="24px"
                        className="candidates-list-title"
                    >
                        {title}
                    </ListTitle>
                </Box>
                {isActive && isCheckable ? (
                    <Checkbox checked={checked} onChange={handleChange} />
                ) : null}
            </ListHeader>
            {isCollapsible ? (
                <Collapse in={isExpanded}>
                    <ListChildrenContainer className="candidates-list-children">
                        {children}
                    </ListChildrenContainer>
                </Collapse>
            ) : (
                <ListChildrenContainer className="candidates-list-children">
                    {children}
                </ListChildrenContainer>
            )}
        </ListContainer>
    )
}

export default CandidatesList
