// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, MenuItem, Select, TextField, Typography} from "@mui/material"
import React, {PropsWithChildren, ReactNode, useId} from "react"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"
import {styled} from "@mui/material/styles"
import {theme} from "../../services/theme"
import {Checkbox} from "@mui/material"
import RadioButtonUncheckedIcon from "@mui/icons-material/RadioButtonUnchecked"
import RadioButtonCheckedIcon from "@mui/icons-material/RadioButtonChecked"
import {faBan, faInfoCircle} from "@fortawesome/free-solid-svg-icons"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {useTranslation} from "react-i18next"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"
import {getOrdinalSuffix} from "./ordinalUtils"

// Type wrapper for MUI icons to work with React 19
const RadioButtonUncheckedIconFixed: React.FC<any> = (props) => {
    const Icon = RadioButtonUncheckedIcon as any
    return <Icon {...props} />
}

const RadioButtonCheckedIconFixed: React.FC<any> = (props) => {
    const Icon = RadioButtonCheckedIcon as any
    return <Icon {...props} />
}

export const UnselectableTypography = styled(Typography)<{component?: React.ElementType}>`
    user-select: none;
`

export const BorderBox = styled("li")<{
    isSelectable: boolean
    hasCategory: boolean
    isInvalidVote: boolean
    isDisabled: boolean
}>`
    border: 2px solid
        ${({hasCategory, isSelectable, theme}) =>
            isSelectable && hasCategory ? theme.palette.white : theme.palette.customGrey.light};
    ${({hasCategory, isInvalidVote, theme}) =>
        hasCategory
            ? `backgroundColor: ${theme.palette.white};`
            : isInvalidVote
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
    ${({isDisabled}) => (isDisabled ? `opacity: 50%;` : "")}
    ${({isSelectable, hasCategory, theme}) =>
        isSelectable
            ? hasCategory
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
    isSelectable?: boolean // Shall the candidate be selectable (Checkbox or Position combo box)?
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
    // Id of the message explaining why the write-in text is rejected, so the
    // field itself can point at it rather than leaving the voter to find it.
    writeInErrorId?: string
    index?: number
    shouldDisable?: boolean
    className?: string
    isPreferentialVote?: boolean
    totalCandidates?: number
    maxVotes?: number
    selectedPosition?: number | null
    handlePreferentialChange?: (value: number | null) => void
}

const Candidate: React.FC<CandidateProps> = ({
    title,
    description,
    isSelectable: isSelectable,
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
    writeInErrorId,
    children,
    shouldDisable,
    index,
    className,
    isPreferentialVote = false,
    totalCandidates = 0,
    maxVotes,
    selectedPosition,
    handlePreferentialChange,
}) => {
    const {t} = useTranslation()
    // The candidate name is the label for every control in the row. Referencing
    // the rendered title with aria-labelledby keeps the visible text and the
    // accessible name in sync even when `title` is rich content rather than a
    // plain string.
    const generatedId = useId()
    const titleId = `${generatedId}-title`
    const positionLabelId = `${generatedId}-position-label`
    const writeInLabelId = `${generatedId}-writein-label`
    const moreInfoLabelId = `${generatedId}-more-info-label`
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

    const maxSelectablePositions =
        typeof maxVotes === "number" && maxVotes > 0
            ? Math.min(maxVotes, totalCandidates)
            : totalCandidates

    const scrollablePreferentialVote = maxSelectablePositions > 4

    // A write-in option usually has no name until the voter types one, so the
    // title alone would leave its checkbox with an empty accessible name. The
    // hidden write-in label is prepended so the control is always named.
    const checkboxLabelIds = isWriteIn ? `${writeInLabelId} ${titleId}` : titleId

    return (
        <BorderBox
            isSelectable={!!isSelectable}
            hasCategory={!!hasCategory}
            isInvalidVote={!!isInvalidVote}
            isDisabled={!!shouldDisable}
            onClick={onClick}
            className={`candidate-item ${className}`}
        >
            <ImageBox className="image-box">{children}</ImageBox>
            <Box flexGrow={2}>
                <UnselectableTypography
                    className="candidate-title"
                    id={titleId}
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
                    component="div"
                    color={theme.palette.customGrey.dark}
                    fontSize="16px"
                    marginTop="4px"
                    marginBottom="4px"
                >
                    {description}
                </UnselectableTypography>
                {isWriteIn ? (
                    <Box>
                        <VisuallyHidden id={writeInLabelId}>{t("a11y.writeInFor")}</VisuallyHidden>
                        <TextField
                            className="candidate-writein-textfield"
                            placeholder={t("candidate.writeInsPlaceholder")}
                            value={writeInValue}
                            onChange={onWriteInTextChange}
                            onClick={handleWriteInClick}
                            error={isInvalidWriteIn || false}
                            slotProps={{
                                htmlInput: {
                                    "aria-labelledby": `${writeInLabelId} ${titleId}`,
                                    "aria-describedby":
                                        isInvalidWriteIn && writeInErrorId
                                            ? writeInErrorId
                                            : undefined,
                                },
                            }}
                        />
                    </Box>
                ) : null}
            </Box>
            {url ? (
                <StyledLink
                    href={url}
                    target="_blank"
                    className="candidate-link"
                    aria-labelledby={`${moreInfoLabelId} ${titleId}`}
                >
                    <FontAwesomeIcon
                        icon={faInfoCircle}
                        size="sm"
                        className="candidate-icon"
                        aria-hidden="true"
                    />
                    <Typography
                        className="candidate-link-text"
                        id={moreInfoLabelId}
                        variant="body2"
                        sx={{margin: "2px 0 0 6px", display: {xs: "none", sm: "block"}}}
                    >
                        {t("candidate.moreInformationLink")}
                    </Typography>
                </StyledLink>
            ) : null}

            {isPreferentialVote ? (
                isSelectable ? (
                    <>
                        <VisuallyHidden id={positionLabelId}>
                            {t("a11y.preferenceLabel")}
                        </VisuallyHidden>
                        <Select
                            displayEmpty
                            disabled={shouldDisable}
                            value={selectedPosition ?? 0}
                            onChange={handlePositionChange}
                            // Must be labelId, not aria-labelledby: MUI puts
                            // labelId on the element that carries
                            // role="combobox", whereas aria-labelledby would
                            // land on the outer wrapper and name nothing.
                            labelId={`${positionLabelId} ${titleId}`}
                            renderValue={(value) => {
                                if (typeof value === "number" && value > 0) {
                                    return getOrdinalSuffix(value, t)
                                }
                                return t("candidate.preferential.position")
                            }}
                            MenuProps={{
                                PaperProps: {
                                    style: {
                                        maxHeight: 200,
                                        overflowY: scrollablePreferentialVote ? "auto" : "visible",
                                    },
                                },
                                autoFocus: false,
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
                            {Array.from({length: maxSelectablePositions}, (_, i) => i + 1).map(
                                (num) => (
                                    <MenuItem key={num} value={num}>
                                        {getOrdinalSuffix(num, t)}
                                    </MenuItem>
                                )
                            )}
                        </Select>
                    </>
                ) : selectedPosition && selectedPosition > 0 ? (
                    <Typography
                        className="candidate-position-label"
                        variant="body2"
                        sx={{
                            minWidth: 52,
                            textAlign: "center",
                            fontWeight: "bold",
                            padding: "4px 8px",
                            borderRadius: "4px",
                            border: `1px solid ${theme.palette.customGrey.light}`,
                            whiteSpace: "nowrap",
                            marginRight: 1,
                        }}
                    >
                        {getOrdinalSuffix(selectedPosition, t)}
                    </Typography>
                ) : null
            ) : isSelectable ? (
                iconCheckboxPolicy === ECandidatesIconCheckboxPolicy.ROUND_CHECKBOX ? (
                    <Checkbox
                        className="candidate-checkbox"
                        slotProps={{
                            input: {
                                "className": "candidate-input",
                                "aria-labelledby": checkboxLabelIds,
                            },
                        }}
                        icon={<RadioButtonUncheckedIconFixed />}
                        checkedIcon={<RadioButtonCheckedIconFixed />}
                        disabled={shouldDisable}
                        checked={checked}
                        onChange={handleChange}
                    />
                ) : (
                    <Checkbox
                        className="candidate-checkbox"
                        slotProps={{
                            input: {
                                "className": "candidate-input",
                                "aria-labelledby": checkboxLabelIds,
                            },
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
