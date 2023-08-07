// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import styled from "@emotion/styled"
import {Box, Typography} from "@mui/material"
import {theme} from "../../../services/theme"

interface ColoredBoxProps {
    backgroundColor?: string
}

const ColorBox = styled(Box)<ColoredBoxProps>`
    height: 40px;
    width: 40px;
    border: 1px solid
        ${({theme, backgroundColor}) =>
            backgroundColor === theme.palette.white
                ? theme.palette.customGrey.light
                : theme.palette.white};
    background-color: ${({backgroundColor}) => backgroundColor || "inherit"};
`

const DescriptorContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
`

const ColorsListContainer = styled(Box)`
    border: 1px solid ${({theme}) => theme.palette.customGrey.light};
    margin: 32px 0;
    padding: 32px;
    border-radius: 8px;
`

const ColoredCell = styled(Box)`
    height: 40px;
    width: 140px;
    display: flex;
    align-items: center;
    justify-content: center;
`

const TH = styled.th`
    width: 20%;
    height: 40px;
    font-weight: normal;
    border-top: 1px solid ${({theme}) => theme.palette.customGrey.light};
`

interface TDProps {
    isHeader: boolean
}

const TD = styled.td<TDProps>`
    width: 20%;
    border-top: ${({theme, isHeader}) =>
        isHeader ? `1px solid ${theme.palette.customGrey.light}` : "none"};
`

interface ColorDescriptorProps {
    title: string
    description: string
    backgroundColor?: string
}

const ColorDescriptor: React.FC<ColorDescriptorProps> = ({title, description, backgroundColor}) => (
    <DescriptorContainer>
        <ColorBox backgroundColor={backgroundColor} />
        <Box sx={{flexGrow: 2}}>
            <Typography mb={0.5}>{title}</Typography>
            <Typography mt={0.5} color={theme.palette.customGrey.main}>
                {description}
            </Typography>
        </Box>
        <Typography>{backgroundColor}</Typography>
    </DescriptorContainer>
)

interface ColorTableCellProps {
    value: string | null
    isHeader: boolean
    color?: string
}

const ColorTableCell: React.FC<ColorTableCellProps> = ({value, isHeader, color}) => (
    <TD isHeader={isHeader || !value}>
        {isHeader ? (
            value
        ) : (
            <ColoredCell sx={{backgroundColor: value, color: color}}>{value}</ColoredCell>
        )}
    </TD>
)

interface ColorsRow {
    values: Array<string | null>
    colors?: Array<string | null>
}

interface ColorsTableProps {
    headers: Array<string | null>
    rows: Array<ColorsRow>
}

const ColorsTable: React.FC<ColorsTableProps> = ({headers, rows}) => (
    <table>
        <tr>
            {headers.map((header, idx) => (
                <TH key={idx}>{header}</TH>
            ))}
        </tr>
        {rows.map((row, rowIdx) => (
            <tr key={rowIdx}>
                {row.values.map((cell, cellIdx) => (
                    <ColorTableCell
                        color={row.colors?.[cellIdx] || undefined}
                        value={cell}
                        isHeader={0 === cellIdx}
                        key={cellIdx}
                    />
                ))}
            </tr>
        ))}
    </table>
)

interface ColorsListProps {
    colors: Array<ColorDescriptorProps>
    colorsTable?: ColorsTableProps
}

const ColorsList: React.FC<ColorsListProps> = ({colors, colorsTable}) => (
    <>
        {colors ? (
            <ColorsListContainer>
                {colors.map((color, idx) => (
                    <ColorDescriptor {...color} key={idx} />
                ))}
            </ColorsListContainer>
        ) : null}
        {colorsTable ? (
            <ColorsListContainer>
                <ColorsTable {...colorsTable} />
            </ColorsListContainer>
        ) : null}
    </>
)

const meta: Meta<typeof ColorsList> = {
    title: "material/Colors",
    component: ColorsList,
    parameters: {
        backgrounds: {
            default: "white",
        },
    },
}

export default meta

type Story = StoryObj<typeof ColorsList>

export const Semantic: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        colors: [
            {
                backgroundColor: theme.palette.lightBackground,
                title: "light-background",
                description: "Light Background",
            },
            {
                backgroundColor: theme.palette.brandColor,
                title: "brand-color",
                description: "Brand color",
            },
            {
                backgroundColor: theme.palette.brandSuccess,
                title: "brand-success",
                description: "Brand success",
            },
            {
                backgroundColor: theme.palette.errorColor,
                title: "error-color",
                description: "Error color",
            },
            {
                backgroundColor: theme.palette.white,
                title: "white",
                description: "White",
            },
            {
                backgroundColor: theme.palette.black,
                title: "black",
                description: "Black",
            },
        ],
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const Content: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        colorsTable: {
            headers: [null, "Light", "Main", "Dark", "Contrast Text"],
            rows: [
                {
                    values: [
                        "red",
                        theme.palette.red.light || null,
                        theme.palette.red.main || null,
                        theme.palette.red.dark || null,
                        null,
                    ],
                },
                {
                    values: [
                        "green",
                        theme.palette.green.light || null,
                        theme.palette.green.main || null,
                        theme.palette.green.dark || null,
                        null,
                    ],
                },
                {
                    values: [
                        "customGreen",
                        theme.palette.customGreen.light || null,
                        theme.palette.customGreen.main || null,
                        theme.palette.customGreen.dark || null,
                        null,
                    ],
                },
                {
                    values: [
                        "yellow",
                        theme.palette.yellow.light || null,
                        theme.palette.yellow.main || null,
                        theme.palette.yellow.dark || null,
                        null,
                    ],
                },
                {
                    values: [
                        "blue",
                        theme.palette.blue.light || null,
                        theme.palette.blue.main || null,
                        theme.palette.blue.dark || null,
                        null,
                    ],
                },
                {
                    values: [
                        "customGrey",
                        theme.palette.customGrey.light || null,
                        theme.palette.customGrey.main || null,
                        theme.palette.customGrey.dark || null,
                        theme.palette.customGrey.contrastText || null,
                    ],
                    colors: [null, null, null, null, theme.palette.white],
                },
            ],
        },
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
