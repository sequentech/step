// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Collapse, Typography} from "@mui/material"
import React, {PropsWithChildren, useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import theme from "../../services/theme"
import {Checkbox} from "@mui/material"
import {faAngleDown, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"

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
    display: grid;
    grid-template-columns: 1fr auto 1fr;
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
    text-align: center;
    font-size: 24px;
`

const CollapseToggleButton = styled(Button)(({theme}) => ({
    "&&": {
        border: "none",
        boxShadow: "none",
    },
    "&&:hover": {
        border: "none",
    },
    "&&:active": {
        border: "none",
    },
    "&&:focus": {
        border: "none",
        outline: `2px solid ${theme.palette.brandSuccess}`,
        outlineOffset: "2px",
    },
}))

export interface CandidatesListProps extends PropsWithChildren {
    title: string
    isActive?: boolean
    isCheckable?: boolean
    checked?: boolean
    setChecked?: (value: boolean) => void
    shouldDisable?: boolean
    isCollapsible?: boolean
    defaultExpanded?: boolean
    collapseToggleAriaLabel?: string
    showCandidatesLabel?: string
    hideCandidatesLabel?: string
    externalExpanded?: boolean
    onExpandedChange?: (expanded: boolean) => void
}

const CandidatesList: React.FC<CandidatesListProps> = ({
    title,
    children,
    isActive,
    isCheckable,
    checked,
    setChecked,
    shouldDisable,
    isCollapsible,
    defaultExpanded,
    collapseToggleAriaLabel,
    showCandidatesLabel,
    hideCandidatesLabel,
    externalExpanded,
    onExpandedChange,
}) => {
    const [isExpanded, setIsExpanded] = useState<boolean>(defaultExpanded ?? true)

    useEffect(() => {
        if (externalExpanded !== undefined) {
            setIsExpanded(externalExpanded)
        }
    }, [externalExpanded])

    const onClick = () => {
        if (isActive && isCheckable && !shouldDisable && setChecked) {
            setChecked(!checked)
        }
    }
    const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        event.stopPropagation()
        if (isActive && isCheckable && !shouldDisable && setChecked) {
            setChecked(event.target.checked)
        }
    }

    const handleToggleCollapse = (event: React.MouseEvent) => {
        event.stopPropagation()
        const newExpanded = !isExpanded
        setIsExpanded(newExpanded)
        onExpandedChange?.(newExpanded)
    }

    return (
        <ListContainer
            isactive={String(!!(isActive && isCheckable && !shouldDisable))}
            onClick={onClick}
            className="candidates-list"
            aria-disabled={shouldDisable}
        >
            <ListHeader className="candidates-list-header">
                <Box>
                    {isCollapsible ? (
                        <CollapseToggleButton
                            variant="secondary"
                            size="small"
                            startIcon={
                                <FontAwesomeIcon icon={isExpanded ? faAngleDown : faAngleRight} />
                            }
                            onClick={handleToggleCollapse}
                            aria-label={collapseToggleAriaLabel}
                            aria-expanded={isExpanded}
                        >
                            {isExpanded ? hideCandidatesLabel : showCandidatesLabel}
                        </CollapseToggleButton>
                    ) : null}
                </Box>
                <ListTitle
                    color={theme.palette.customGrey.contrastText}
                    fontSize="24px"
                    className="candidates-list-title"
                >
                    {title}
                </ListTitle>
                <Box sx={{display: "flex", justifyContent: "flex-end"}}>
                    {isActive && isCheckable ? (
                        <Checkbox
                            checked={checked}
                            onChange={handleChange}
                            disabled={shouldDisable}
                        />
                    ) : null}
                </Box>
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
