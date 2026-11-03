// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren} from "react"
import {styled} from "@mui/material/styles"
import {faCheck} from "@fortawesome/free-solid-svg-icons"
import {Icon, VisuallyHidden} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"

const MemberItem = styled("li")({
    "display": "flex",
    "alignItems": "flex-start",
    "gap": "7px",
    "listStyle": "none",
    "margin": "0 -6px",
    "padding": "3px 6px",
    "borderRadius": "4px",
    "lineHeight": 1.5,
    "minWidth": 0,
    "overflowWrap": "anywhere",
    "& .slate-member-check": {
        flex: "none",
        width: "16px",
        height: "21px",
        fontSize: "13px",
        visibility: "hidden",
        color: "#047857",
    },
    "& .slate-member-name": {
        minWidth: 0,
        fontSize: "14px",
    },
    "&.slate-member-selected": {
        backgroundColor: "#eaf7f1",
        color: "#065f46",
    },
    "&.slate-member-selected .slate-member-check": {
        visibility: "visible",
    },
    "&.slate-member-selected .slate-member-name": {
        fontWeight: 600,
    },
})

export interface SlateMemberProps extends PropsWithChildren {
    selected: boolean
    candidateId?: string
    className?: string
}

export const SlateMember: React.FC<SlateMemberProps> = ({
    selected,
    candidateId,
    className,
    children,
}) => {
    const {t} = useTranslation()

    return (
        <MemberItem
            className={["slate-member", selected ? "slate-member-selected" : "", className ?? ""]
                .filter(Boolean)
                .join(" ")}
            data-candidate-id={candidateId}
        >
            <Icon className="slate-member-check" icon={faCheck} aria-hidden="true" />
            <span className="slate-member-name">{children}</span>
            {selected && (
                <VisuallyHidden className="slate-member-selected-label" component="span">
                    {t("slates.selection.selected")}
                </VisuallyHidden>
            )}
        </MemberItem>
    )
}
