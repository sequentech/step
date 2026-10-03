// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren} from "react"
import {styled} from "@mui/material/styles"
import {faCheck} from "@fortawesome/free-solid-svg-icons"
import {Icon, VisuallyHidden} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"

const MemberItem = styled("li")(({theme}) => ({
    "display": "flex",
    "alignItems": "baseline",
    "gap": "8px",
    "listStyle": "none",
    "margin": "0 -6px",
    "padding": "3px 6px 3px 29px",
    "borderRadius": "4px",
    "overflowWrap": "anywhere",
    "&.slate-member-selected": {
        paddingLeft: "6px",
        fontWeight: 700,
        color: theme.palette.green.main,
        backgroundColor: theme.palette.green.light,
    },
    "& .slate-member-check": {
        flex: "0 0 15px",
        fontSize: "13px",
    },
}))

export interface SlateMemberProps extends PropsWithChildren {
    selected: boolean
    className?: string
}

export const SlateMember: React.FC<SlateMemberProps> = ({selected, className, children}) => {
    const {t} = useTranslation()

    return (
        <MemberItem
            className={["slate-member", selected ? "slate-member-selected" : "", className ?? ""]
                .filter(Boolean)
                .join(" ")}
        >
            {selected && <Icon className="slate-member-check" icon={faCheck} aria-hidden="true" />}
            <span className="slate-member-name">{children}</span>
            {selected && (
                <VisuallyHidden component="span">{t("slates.selection.selected")}</VisuallyHidden>
            )}
        </MemberItem>
    )
}
