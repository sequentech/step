// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import * as React from "react"
import Button from "@mui/material/Button"
import Menu from "@mui/material/Menu"
import MenuItem from "@mui/material/MenuItem"
import {useTranslation} from "react-i18next"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faLanguage, faCaretDown} from "@fortawesome/free-solid-svg-icons"
import {getLanguages} from "../../services/i18n"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"

interface ButtonProps {
    isactive?: string
}

const StyledButton = styled(Button)<ButtonProps>(
    ({isactive}) => `
    gap: 10px;
    color: black;
    background-color: ${"true" === isactive ? "#eee" : "inherit"};
`
)

const LanguageMenu: React.FC = () => {
    const {t, i18n} = useTranslation()
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const open = Boolean(anchorEl)
    const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
        setAnchorEl(event.currentTarget)
    }
    const handleClose = () => {
        setAnchorEl(null)
    }
    const changeLanguage = async (lang: string) => {
        handleClose()
        await i18n.changeLanguage(lang)
    }

    const languagesList = getLanguages(i18n)

    return (
        <Box>
            <StyledButton
                id="lang-button"
                variant="actionbar"
                data-testid="lang-button-test"
                aria-controls={open ? "lang-menu" : undefined}
                aria-haspopup="true"
                aria-expanded={open ? "true" : undefined}
                onClick={handleClick}
                isactive={String(open)}
            >
                <FontAwesomeIcon icon={faLanguage} size="lg" />
                <Box component="span" sx={{display: {xs: "none", md: "block"}}}>
                    {t("language")}
                </Box>
                <FontAwesomeIcon icon={faCaretDown} size="lg" />
            </StyledButton>
            <Menu
                id="lang-menu"
                data-testid="lang-menu-test"
                anchorEl={anchorEl}
                open={open}
                onClose={handleClose}
                MenuListProps={{
                    "aria-labelledby": "lang-button",
                }}
            >
                {languagesList.map((language) => (
                    <MenuItem
                        onClick={() => changeLanguage(language)}
                        key={`menu-language-${language}`}
                    >
                        {t("language", {lng: language})}
                    </MenuItem>
                ))}
            </Menu>
        </Box>
    )
}

export default LanguageMenu
