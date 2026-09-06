// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Collapse, Typography} from "@mui/material"
import React, {PropsWithChildren, useEffect, useId, useState} from "react"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"
import {useTranslation} from "react-i18next"
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
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        width: initial;
    }
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
    align-items: center;
    flex-wrap: nowrap;
    gap: 12px;
    width: 100%;
`

const ListTitleSection = styled(Box)`
    display: flex;
    align-items: center;
    flex: 1 1 auto;
    gap: 8px;
    min-width: 0;
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

const ListTitle = styled(Typography)<{component?: React.ElementType}>`
    flex: 1 1 auto;
    min-width: 0;
    text-align: left;
    font-size: 24px;
    margin: 0;
`

const CollapseToggleButton = styled(Button)(({theme}) => ({
    "flexShrink": 0,
    "whiteSpace": "nowrap",
    "& > span:first-of-type": {
        margin: 0,
    },
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
        outlineOffset: "-4px",
    },
}))

const CollapseToggleText = styled("span")(({theme}) => ({
    [theme.breakpoints.down("sm")]: {
        display: "none",
    },
}))

const SelectedCandidatesLabel = styled("span")`
    color: ${theme.palette.customGrey.contrastText};
    font-size: 14px;
    line-height: 1.2;
    text-align: right;
    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        width: min-content;
    }

    /* The element stays mounted while empty so that it is already a live region
       when the count appears in it. It is still a flex item though, so cancel the
       row gap it would otherwise add next to the checkbox. */
    &:empty {
        margin-inline-end: -${({theme}) => theme.spacing(1)};
    }
`

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
    selectedCandidatesLabel?: string
    externalExpanded?: boolean
    onExpandedChange?: (expanded: boolean) => void
    // Heading element for the list title. Left unset the title stays a paragraph,
    // which is what consumers outside the voting portal still render; the voting
    // portal passes the level that fits its contest heading hierarchy.
    titleComponent?: React.ElementType
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
    selectedCandidatesLabel,
    externalExpanded,
    onExpandedChange,
    titleComponent,
}) => {
    const [isExpanded, setIsExpanded] = useState<boolean>(defaultExpanded ?? true)
    const {t} = useTranslation()
    // The list title labels the "select the whole list" checkbox, and the
    // collapse toggle needs to point at the panel it controls.
    const generatedId = useId()
    const titleId = `${generatedId}-title`
    const selectLabelId = `${generatedId}-select-label`
    const panelId = `${generatedId}-panel`

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

    const collapseLabel = isExpanded ? hideCandidatesLabel : showCandidatesLabel

    return (
        <ListContainer
            isactive={String(!!(isActive && isCheckable && !shouldDisable))}
            onClick={onClick}
            className="candidates-list"
        >
            <ListHeader className="candidates-list-header">
                <ListTitleSection>
                    {isCollapsible ? (
                        <CollapseToggleButton
                            className="candidates-list-toggle"
                            variant="secondary"
                            size="small"
                            startIcon={
                                <FontAwesomeIcon icon={isExpanded ? faAngleDown : faAngleRight} />
                            }
                            onClick={handleToggleCollapse}
                            aria-label={collapseToggleAriaLabel ?? collapseLabel}
                            aria-expanded={isExpanded}
                            aria-controls={panelId}
                        >
                            <CollapseToggleText>{collapseLabel}</CollapseToggleText>
                        </CollapseToggleButton>
                    ) : null}
                    <ListTitle
                        color={theme.palette.customGrey.contrastText}
                        fontSize="24px"
                        className="candidates-list-title"
                        component={titleComponent}
                        id={titleId}
                    >
                        {title}
                    </ListTitle>
                </ListTitleSection>
                <Box
                    sx={(muiTheme) => ({
                        display: "flex",
                        justifyContent: "flex-end",
                        alignItems: "center",
                        flexWrap: "wrap",
                        flexShrink: 0,
                        gap: 1,
                        [muiTheme.breakpoints.down("sm")]: {
                            width: "min-content",
                        },
                    })}
                >
                    {/* Stays mounted while collapsible so the changing count is
                        announced; a region added at the same time as its text is
                        not reliably read out. */}
                    {isCollapsible ? (
                        <SelectedCandidatesLabel
                            className="candidates-selected-count"
                            role="status"
                        >
                            {!isExpanded && selectedCandidatesLabel ? selectedCandidatesLabel : ""}
                        </SelectedCandidatesLabel>
                    ) : null}
                    {isActive && isCheckable ? (
                        <>
                            <VisuallyHidden id={selectLabelId}>
                                {t("a11y.selectList")}
                            </VisuallyHidden>
                            <Checkbox
                                className="candidates-list-checkbox"
                                checked={checked}
                                onChange={handleChange}
                                disabled={shouldDisable}
                                slotProps={{
                                    input: {
                                        "aria-labelledby": `${selectLabelId} ${titleId}`,
                                    },
                                }}
                            />
                        </>
                    ) : null}
                </Box>
            </ListHeader>
            {isCollapsible ? (
                <Collapse in={isExpanded}>
                    <ListChildrenContainer
                        className="candidates-list-children"
                        id={panelId}
                        role="list"
                    >
                        {children}
                    </ListChildrenContainer>
                </Collapse>
            ) : (
                <ListChildrenContainer
                    className="candidates-list-children"
                    id={panelId}
                    role="list"
                >
                    {children}
                </ListChildrenContainer>
            )}
        </ListContainer>
    )
}

export default CandidatesList
