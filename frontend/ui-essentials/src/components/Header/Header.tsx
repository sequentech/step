// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Image from "mui-image"
import LanguageMenu from "../LanguageMenu/LanguageMenu"
import PageBanner from "../PageBanner/PageBanner"
import PageLimit from "../PageLimit/PageLimit"
import {theme} from "../../services/theme"
import LogoImg from "../../../public/Sequent_logo.svg"
import styled from "@emotion/styled"
import {Box} from "@mui/material"
import Version from "../Version/Version"
import LogoutButton from "../LogoutButton/LogoutButton"

const HeaderWrapper = styled(PageBanner)`
    background-color: ${theme.palette.lightBackground};
    padding: 16px 0;
    font-size: 16px;

    @media (max-width: ${theme.breakpoints.values.lg}px) {
        padding: 9px;
    }
`

const StyledLink = styled.a`
    max-height: 100%;
    max-width: 50%;
`

const StyledImage = styled(Image)`
    max-height: 100%;
`

export interface HeaderProps {
    logoutFn?: () => void
}

const Header: React.FC<HeaderProps> = ({logoutFn}) => (
    <HeaderWrapper className="header-class" sx={{backgroundColor: theme.palette.lightBackground}}>
        <PageLimit maxWidth="lg" sx={{height: {xs: "37px", md: "47px"}}}>
            <PageBanner direction="row" sx={{height: "100%"}}>
                <StyledLink href="//sequentech.io/" target="_blank">
                    <StyledImage src={LogoImg} duration={100} alt="Logo Image" />
                </StyledLink>
                <Box display="flex" alignItems="center" sx={{gap: {xs: "11px", lg: "31px"}}}>
                    <Version version={{main: "7.1.0"}} />
                    <LanguageMenu />
                    {logoutFn ? <LogoutButton logoutFn={logoutFn} /> : null}
                </Box>
            </PageBanner>
        </PageLimit>
    </HeaderWrapper>
)

export default Header
