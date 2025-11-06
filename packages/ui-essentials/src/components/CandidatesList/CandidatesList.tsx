// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React, {PropsWithChildren} from "react"
import {styled} from "@mui/material/styles"
import theme from "../../services/theme"
import {Checkbox} from "@mui/material"
import emotionStyled from "@emotion/styled"

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
`

const ListChildrenContainer = emotionStyled.ul`
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
}

const CandidatesList: React.FC<CandidatesListProps> = ({
    title,
    children,
    isActive,
    isCheckable,
    checked,
    setChecked,
}) => {
    const onClick = () => {
        if (isActive && isCheckable && setChecked) {
            setChecked(!checked)
        }
    }
    const handleChange = (event: React.ChangeEvent<HTMLInputElement>) =>
        isActive && isCheckable && setChecked && setChecked(event.target.checked)

    return (
        <ListContainer
            isactive={String(!!(isActive && isCheckable))}
            onClick={onClick}
            className="candidates-list"
        >
            <ListHeader className="candidates-list-header">
                <Box>
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
            <ListChildrenContainer className="candidates-list-children">
                {children}
            </ListChildrenContainer>
        </ListContainer>
    )
}

export default CandidatesList
