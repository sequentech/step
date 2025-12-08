// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, ReactNode} from "react"
import {Box} from "@mui/material"
// import Image from "mui-image"
// import CandidateImg from "../../../../public/example_candidate.jpg"
import {Candidate} from "@sequentech/ui-essentials"
import {fireEvent, within} from "@testing-library/react"

export interface CandidateProps extends PropsWithChildren {
    title: string | ReactNode
    description?: string | ReactNode
    isActive?: boolean
    isInvalidVote?: boolean
    checked?: boolean
    hasCategory?: boolean
    url?: string
    setChecked?: (value: boolean) => void
    isWriteIn?: boolean
    writeInValue?: string
    setWriteInText?: (value: string) => void
    isInvalidWriteIn?: boolean
    index?: number
}

// const CandidateWrapper: React.FC<CandidateProps> = ({
//     className,
//     ...props
// }) => (
//     <Box className={className}>
//         <Candidate {...props} />
//     </Box>
// )

const meta: any = {
    title: "Candidate",
    component: Candidate,
}

export default meta

// export const Basic: any = Object.assign(() => <Candidate
//     title="Micky Mouse"
//     description="Candidate Description"
//     isActive={true}
//     checked={true}
//     url="https://google.com" />, {
//         test: async (browser, {component, result}) => {
//             await expect(component).to.be.visible;
//             await expect(component).to.have.text("Micky Mouse");
//         }
//     })

// export const ReadOnly: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: "Micky Mouse",
//         description: "Candidate Description",
//         isActive: false,
//         checked: true,
//         url: "https://google.com",
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const NoImage: any = {
//     args: {
//         title: "Micky Mouse",
//         description: "Candidate Description",
//         isActive: true,
//         url: "https://google.com",
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const NoDescription: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: "Micky Mouse",
//         isActive: true,
//         url: "https://google.com",
//         checked: false,
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const OnlyTitle: any = {
//     args: {
//         title: "Micky Mouse",
//         isActive: true,
//         checked: false,
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const LongDescription: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: "Micky Mouse",
//         description:
//             "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const LongTitle: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
//         description: "Candidate Description",
//         isActive: true,
//         url: "https://google.com",
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const WithHtml: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: (
//             <>
//                 Micky <b>Mouse</b>
//             </>
//         ),
//         description: (
//             <>
//                 Candidate <b>description</b>
//             </>
//         ),
//         isActive: true,
//         url: "https://google.com",
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const Hover: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: "Micky Mouse",
//         description: "Candidate Description",
//         className: "hover",
//         isActive: true,
//         url: "https://google.com",
//     },
//     parameters: {
//         pseudo: {
//             hover: [".hover"],
//             active: [".active"],
//             focus: [".focus"],
//         },
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const Active: any = {
//     args: {
//         children: <Image src={CandidateImg} duration={100} />,
//         title: "Micky Mouse",
//         description: "Candidate Description",
//         className: "hover active",
//         isActive: true,
//         url: "https://google.com",
//     },
//     parameters: {
//         pseudo: {
//             hover: [".hover"],
//             active: [".active"],
//             focus: [".focus"],
//         },
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const WriteInSimple: any = {
//     args: {
//         title: "",
//         description: "",
//         isActive: true,
//         isWriteIn: true,
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const WriteInInvalid: any = {
//     args: {
//         title: "",
//         description: "",
//         isActive: true,
//         isWriteIn: true,
//         writeInValue: "John Connor",
//         isInvalidWriteIn: true,
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const WriteInFields: any = {
//     args: {
//         title: "",
//         description: "",
//         isActive: true,
//         isWriteIn: true,
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }

// export const InvalidVote: any = {
//     args: {
//         title: "Micky Mouse",
//         isActive: true,
//         isInvalidVote: true,
//         checked: false,
//     },
//     parameters: {
//         viewport: {
//             disable: true,
//         },
//     },
// }
