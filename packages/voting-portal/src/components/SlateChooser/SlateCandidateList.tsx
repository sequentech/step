// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren, useId} from "react"
import {styled, useTheme} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {faChevronDown, faChevronUp} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {useAppDispatch, useAppSelector} from "../../store/hooks"
import {selectSlateListExpanded, setSlateListExpanded} from "../../store/extra/extraSlice"

const Toggle = styled("button")(({theme}) => ({
    "display": "flex",
    "alignItems": "center",
    "justifyContent": "space-between",
    "width": "100%",
    "minHeight": "44px",
    "padding": "0",
    "border": "none",
    "background": "none",
    "font": "inherit",
    "fontWeight": 500,
    "color": theme.palette.brandColor,
    "textAlign": "start",
    "cursor": "pointer",
    "&:focus-visible": {
        outline: `2px solid ${theme.palette.brandColor}`,
        outlineOffset: "2px",
    },
}))

// On desktop the offices are laid out by the slate card, across cards.
const List = styled("div")(({theme}) => ({
    [theme.breakpoints.up("sm")]: {
        display: "contents",
    },
}))

export interface SlateCandidateListProps extends PropsWithChildren {
    electionId: string
    slateId: string
    defaultExpanded: boolean
}

/**
 * The candidates of one slate. Always visible on desktop; on a phone the
 * voter can collapse it, starting from the state the election configures.
 * The toggle is kept for the ballot session and is never a vote.
 */
export const SlateCandidateList: React.FC<SlateCandidateListProps> = ({
    electionId,
    slateId,
    defaultExpanded,
    children,
}) => {
    const {t} = useTranslation()
    const theme = useTheme()
    const dispatch = useAppDispatch()
    const listId = useId()
    const isPhone = useMediaQuery(theme.breakpoints.down("sm"))
    const stored = useAppSelector(selectSlateListExpanded(electionId, slateId))
    const expanded = !isPhone || (stored ?? defaultExpanded)

    return (
        <>
            {isPhone && (
                <Toggle
                    type="button"
                    className="slate-candidate-list-toggle"
                    aria-expanded={expanded}
                    aria-controls={listId}
                    onClick={() =>
                        dispatch(setSlateListExpanded({electionId, slateId, expanded: !expanded}))
                    }
                >
                    {t(expanded ? "slates.candidateList.hide" : "slates.candidateList.show")}
                    <Icon
                        className="slate-candidate-list-toggle-icon"
                        icon={expanded ? faChevronUp : faChevronDown}
                        aria-hidden="true"
                    />
                </Toggle>
            )}
            <List id={listId} className="slate-candidate-list" hidden={!expanded}>
                {children}
            </List>
        </>
    )
}
