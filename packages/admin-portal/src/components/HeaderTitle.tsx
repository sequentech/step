// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"

import {useTranslation} from "react-i18next"

const HeaderStyles = {
    Wrapper: styled("div")`
        display: flex;
        flex-direction: column;
        padding: var(--2, 16px);
        align-items: left;
    `,
    Title: styled("div")`
        color: rgba(0, 0, 0, 0.87);
        font-size: 24px;
        font-family: Roboto;
        font-weight: 700;
        line-height: 32.02px;
        word-wrap: break-word;
    `,
    SubTitle: styled("div")`
        color: rgba(0, 0, 0, 0.6);
        font-size: 14px;
        font-family: Roboto;
        font-weight: 400;
        line-height: 20.02px;
        letter-spacing: 0.17px;
        word-wrap: break-word;
    `,
}

type HeaderProps = {
    title: string
    subtitle: string
}

export const HeaderTitle: React.FC<HeaderProps> = ({title, subtitle}) => {
    const {t} = useTranslation()

    return (
        <HeaderStyles.Wrapper>
            <HeaderStyles.Title>{t(title)}</HeaderStyles.Title>
            <HeaderStyles.SubTitle>{t(subtitle)}</HeaderStyles.SubTitle>
        </HeaderStyles.Wrapper>
    )
}
