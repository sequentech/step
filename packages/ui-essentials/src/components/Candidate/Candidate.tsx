// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, FormControl, InputLabel, MenuItem, Select, TextField, Typography} from "@mui/material"
import React, {PropsWithChildren, ReactNode} from "react"
import {styled} from "@mui/material/styles"
import {theme} from "../../services/theme"
import {Checkbox} from "@mui/material"
import RadioButtonUncheckedIcon from "@mui/icons-material/RadioButtonUnchecked"
import RadioButtonCheckedIcon from "@mui/icons-material/RadioButtonChecked"
import {faBan, faInfoCircle} from "@fortawesome/free-solid-svg-icons"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {useTranslation} from "react-i18next"
import {isString, ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"

// Type wrapper for MUI icons to work with React 19
const RadioButtonUncheckedIconFixed: React.FC<any> = (props) => {
    const Icon = RadioButtonUncheckedIcon as any
    return <Icon {...props} />
}

const RadioButtonCheckedIconFixed: React.FC<any> = (props) => {
    const Icon = RadioButtonCheckedIcon as any
    return <Icon {...props} />
}

const UnselectableTypography = styled(Typography)`
    user-select: none;
`

const BorderBox = styled("li")<{
    isactive: string
    hasCategory: string
    isinvalidvote: string
    isdisabled: string
}>`
    border: 2px solid
        ${({hasCategory: hascategory, isactive, theme}) =>
            isactive === "true" && hascategory === "true"
                ? theme.palette.white
                : theme.palette.customGrey.light};
    ${({hasCategory, isinvalidvote, theme}) =>
        hasCategory === "true"
            ? `backgroundColor: ${theme.palette.white};`
            : isinvalidvote === "true"
              ? `backgroundColor: ${theme.palette.lightBackground};`
              : ""}
    border-radius: 10px;
    break-inside: avoid;
    padding: 8px;
    height: 64px;
    display: flex;
    flex-direction: row;
    gap: 10px;
    align-items: center;
    flex-grow: 2;
    transition: all 0.2s ease;
    ${({isdisabled}) => (isdisabled === "true" ? `opacity: 50%;` : "")}
    ${({isactive, hasCategory, theme}) =>
        isactive === "true"
            ? hasCategory === "true"
                ? `
                    box-shadow: 0 5px 5px rgba(0, 0, 0, 0.5);
                    &:hover {
                        cursor: pointer;
                        border-color: ${theme.palette.customGrey.light};
                    }
                    &:active {
                        background-color: #eee;
                    }
                `
                : `
                    &:hover {
                        cursor: pointer;
                        box-shadow: 0 5px 5px rgba(0, 0, 0, 0.5);
                    }
                    &:active {
                        background-color: #eee;
                    }
                `
            : ""}
`

const ImageBox = styled(Box)`
    display: flex;
    width: 64px;
    height: 64px;
    position: relative;
    flex-shrink: 0;
`

const StyledLink = styled("a")`
    text-decoration: underline;
    font-weight: normal;
    &:hover {
        text-decoration: none;
    }
    display: flex;
    flex: direction: row;
    align-items: center;
    color: ${({theme}) => theme.palette.brandColor};
`

export interface CandidateProps extends PropsWithChildren {
    title: string | ReactNode
    description?: string | ReactNode
    isActive?: boolean // Shall the candidate be selectable (Checkbox or Position combo box)?
    isInvalidVote?: boolean
    checked?: boolean
    iconCheckboxPolicy?: ECandidatesIconCheckboxPolicy
    hasCategory?: boolean
    url?: string
    setChecked?: (value: boolean) => void
    isWriteIn?: boolean
    writeInValue?: string
    setWriteInText?: (value: string) => void
    isInvalidWriteIn?: boolean
    index?: number
    shouldDisable?: boolean
    className?: string
    isPreferentialVote?: boolean
    totalCandidates?: number
    selectedPosition?: number | null
    handlePreferentialChange?: (value: number | null) => void
}

const Candidate: React.FC<CandidateProps> = ({
    title,
    description,
    isActive,
    isInvalidVote,
    checked,
    iconCheckboxPolicy,
    hasCategory,
    url,
    setChecked,
    isWriteIn,
    writeInValue,
    setWriteInText,
    isInvalidWriteIn,
    children,
    shouldDisable,
    index,
    className,
    isPreferentialVote = false,
    totalCandidates = 0,
    selectedPosition,
    handlePreferentialChange,
}) => {
    const {t} = useTranslation()
    const onClick: React.MouseEventHandler<HTMLLIElement> = (event) => {
        event.stopPropagation()
        if (!shouldDisable && setChecked) {
            setChecked(!checked)
        }
    }
    const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        event.stopPropagation()
        if (setChecked) {
            setChecked(event.target.checked)
        }
    }

    const onWriteInTextChange: React.ChangeEventHandler<HTMLInputElement> = (event) => {
        setWriteInText && setWriteInText(event.target.value)
    }

    const handleWriteInClick: React.MouseEventHandler<HTMLDivElement> = (event) => {
        event.stopPropagation()
    }

    const handlePositionChange = (event: any) => {
        event.stopPropagation()
        if (handlePreferentialChange) {
            const value = event.target.value
            handlePreferentialChange(value === "" ? null : value)
        }
    }

    const getOrdinalSuffix = (num: number): string => {
        if (num === 1) return `${num}${t("candidate.preferential.ordinals.first")}`
        if (num === 2) return `${num}${t("candidate.preferential.ordinals.second")}`
        if (num === 3) return `${num}${t("candidate.preferential.ordinals.third")}`
        return `${num}${t("candidate.preferential.ordinals.other")}`
    }

    console.log("hasCategory", hasCategory)
    return (
        <BorderBox
            isactive={String(!!isActive)}
            hasCategory={String(!!hasCategory)}
            isinvalidvote={String(!!isInvalidVote)}
            isdisabled={String(!!shouldDisable)}
            onClick={onClick}
            className={`candidate-item ${className}`}
        >
            <ImageBox className="image-box">{children}</ImageBox>
            <Box flexGrow={2}>
                <UnselectableTypography
                    className="candidate-title"
                    fontWeight="bold"
                    fontSize="16px"
                    lineHeight="22px"
                    marginTop="4px"
                    marginBottom="4px"
                    color={theme.palette.customGrey.contrastText}
                >
                    {title}
                </UnselectableTypography>
                <UnselectableTypography
                    className="candidate-description"
                    color={theme.palette.customGrey.dark}
                    fontSize="16px"
                    marginTop="4px"
                    marginBottom="4px"
                >
                    {description}
                </UnselectableTypography>
                {isWriteIn ? (
                    <Box>
                        <TextField
                            className="candidate-writein-textfield"
                            placeholder={t("candidate.writeInsPlaceholder")}
                            InputLabelProps={{shrink: true}}
                            value={writeInValue}
                            onChange={onWriteInTextChange}
                            onClick={handleWriteInClick}
                            error={isInvalidWriteIn || false}
                        />
                    </Box>
                ) : null}
            </Box>
            {url ? (
                <StyledLink href={url} target="_blank" className="candidate-link">
                    <FontAwesomeIcon icon={faInfoCircle} size="sm" className="candidate-icon" />
                    <Typography
                        className="candidate-link-text"
                        variant="body2"
                        sx={{margin: "2px 0 0 6px", display: {xs: "none", sm: "block"}}}
                    >
                        {t("candidate.moreInformationLink")}
                    </Typography>
                </StyledLink>
            ) : null}

            {isPreferentialVote ? (
                <Select
                    displayEmpty
                    value={selectedPosition ?? 0}
                    onChange={handlePositionChange}
                    disabled={!isActive}
                    renderValue={(value) => {
                        if (typeof value === "number" && value > 0) {
                            return getOrdinalSuffix(value)
                        }
                        return t("candidate.preferential.position")
                    }}
                    sx={{
                        "minWidth": 120,
                        "marginRight": 1,
                        "& .MuiSelect-select": {
                            paddingTop: "6px",
                            paddingBottom: "6px",
                        },
                    }}
                    className="candidate-position-select"
                >
                    <MenuItem value={0}>
                        <em>{t("candidate.preferential.none")}</em>
                    </MenuItem>
                    {Array.from({length: totalCandidates}, (_, i) => i + 1).map((num) => (
                        <MenuItem key={num} value={num}>
                            {getOrdinalSuffix(num)}
                        </MenuItem>
                    ))}
                </Select>
            ) : isActive ? (
                iconCheckboxPolicy === ECandidatesIconCheckboxPolicy.ROUND_CHECKBOX ? (
                    <Checkbox
                        inputProps={{
                            "className": "candidate-input",
                            "aria-label": isString(title) ? title : "",
                        }}
                        icon={<RadioButtonUncheckedIconFixed />}
                        checkedIcon={<RadioButtonCheckedIconFixed />}
                        disabled={shouldDisable}
                        checked={checked}
                        onChange={handleChange}
                    />
                ) : (
                    <Checkbox
                        inputProps={{
                            "className": "candidate-input",
                            "aria-label": isString(title) ? title : "",
                        }}
                        disabled={shouldDisable}
                        checked={checked}
                        onChange={handleChange}
                    />
                )
            ) : null}
        </BorderBox>
    )
}

export default Candidate
