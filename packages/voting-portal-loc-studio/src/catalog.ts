// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface SceneVariant {
    id: string
    label: string
    description: string
    keyPrefixes: string[]
    extraKeys?: string[]
}

export interface SceneDefinition {
    id: string
    label: string
    variants: SceneVariant[]
}

const sharedChromeKeys = [
    "footer.poweredBy",
    "header.profile",
    "header.welcome",
    "logout.buttonText",
    "version.header",
    "hash.header",
]

const ballotChromeKeys = ["votingScreen", "candidatesList", "candidate", "breadcrumbSteps", ...sharedChromeKeys]

export const VOTE_ROUTE_SCENES = [
    "voting",
    "write-in",
    "overvote",
    "undervote",
    "blank",
    "invalid",
] as const

export type VoteRouteSceneId = (typeof VOTE_ROUTE_SCENES)[number]

export const isVoteRouteScene = (sceneId: string): sceneId is VoteRouteSceneId =>
    (VOTE_ROUTE_SCENES as readonly string[]).includes(sceneId)

export const SCENES: SceneDefinition[] = [
    {
        id: "election-list",
        label: "Ballot list",
        variants: [
            {
                id: "default",
                label: "Open ballots",
                description: "Chooser with open and closed ballots",
                keyPrefixes: [
                    "electionSelectionScreen",
                    "selectElection",
                    "breadcrumbSteps",
                    "materials",
                ],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "help",
                label: "Help dialog",
                description: "About the ballot list",
                keyPrefixes: ["electionSelectionScreen.chooserHelpDialog"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "demo",
                label: "Demo dialog",
                description: "Demo voting booth warning",
                keyPrefixes: ["electionSelectionScreen.demoDialog"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "errors",
                label: "Errors & alerts",
                description: "Network, unpublished, and empty-list messages",
                keyPrefixes: ["electionSelectionScreen.errors", "electionSelectionScreen.alerts"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "start",
        label: "How to vote",
        variants: [
            {
                id: "default",
                label: "Instructions",
                description: "Start screen with voting steps",
                keyPrefixes: ["startScreen", "breadcrumbSteps"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "demo",
                label: "Demo dialog",
                description: "Demo booth warning on start",
                keyPrefixes: ["startScreen", "electionSelectionScreen.demoDialog"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "voting",
        label: "Ballot",
        variants: [
            {
                id: "default",
                label: "Contest",
                description: "Candidate selection with chrome buttons",
                keyPrefixes: ballotChromeKeys,
                extraKeys: [],
            },
            {
                id: "help",
                label: "Help dialog",
                description: "About this screen",
                keyPrefixes: ["votingScreen.ballotHelpDialog"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "write-in",
        label: "Write-in",
        variants: [
            {
                id: "default",
                label: "Write-in page",
                description: "Write-in candidate fields on a dedicated ballot page",
                keyPrefixes: ["votingScreen", "candidatesList", "candidate", "breadcrumbSteps"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "overvote",
        label: "Overvote",
        variants: [
            {
                id: "default",
                label: "Overvote warning",
                description: "Overvote blocking dialog",
                keyPrefixes: ["votingScreen.nonVotedDialog", "errors.implicit"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "undervote",
        label: "Undervote",
        variants: [
            {
                id: "default",
                label: "Undervote warning",
                description: "Undervote allowed with review dialog",
                keyPrefixes: ["votingScreen.warningDialog", "errors.implicit"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "blank",
        label: "Blank vote",
        variants: [
            {
                id: "default",
                label: "Blank vote",
                description: "Blank vote warning when no choices are selected",
                keyPrefixes: [
                    "candidate.blankVote",
                    "errors.implicit.blankVote",
                    "votingScreen.warningDialog",
                ],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "invalid",
        label: "Invalid vote",
        variants: [
            {
                id: "default",
                label: "Invalid vote",
                description: "Explicit invalid vote selected",
                keyPrefixes: ["errors.explicit", "candidate", "votingScreen"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "review",
        label: "Review",
        variants: [
            {
                id: "default",
                label: "Review ballot",
                description: "Read-only selections before cast",
                keyPrefixes: ["reviewScreen", "ballotHash", "breadcrumbSteps"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "confirm",
                label: "Confirm cast",
                description: "Cast confirmation dialog",
                keyPrefixes: ["reviewScreen.confirmCastVoteDialog"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "audit-help",
                label: "Audit help",
                description: "Spoil-ballot warning before audit",
                keyPrefixes: [
                    "reviewScreen.auditBallotHelpDialog",
                    "reviewScreen.ballotIdHelpDialog",
                ],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "error",
                label: "Cast error",
                description: "Failed cast with error banner",
                keyPrefixes: ["reviewScreen.error"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "confirmation",
        label: "Confirmation",
        variants: [
            {
                id: "default",
                label: "Vote cast",
                description: "Success with ballot ID",
                keyPrefixes: ["confirmationScreen", "ballotHash", "breadcrumbSteps"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "help",
                label: "Help dialogs",
                description: "Confirmation and ballot ID help",
                keyPrefixes: [
                    "confirmationScreen.confirmationHelpDialog",
                    "confirmationScreen.ballotIdHelpDialog",
                ],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "demo",
                label: "Demo mode",
                description: "Print and tracker disabled in demo",
                keyPrefixes: [
                    "confirmationScreen.demoPrintDialog",
                    "confirmationScreen.demoBallotUrlDialog",
                    "confirmationScreen.demoQRText",
                    "confirmationScreen.ballotIdDemoHelpDialog",
                ],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "audit",
        label: "Audit",
        variants: [
            {
                id: "default",
                label: "Check ballot",
                description: "Audit instructions and spoil warning",
                keyPrefixes: ["auditScreen", "breadcrumbSteps"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "help",
                label: "Step help",
                description: "Download and verifier help dialogs",
                keyPrefixes: ["auditScreen.step1HelpDialog", "auditScreen.step2HelpDialog"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "ballot-locator",
        label: "Ballot finder",
        variants: [
            {
                id: "lookup",
                label: "Lookup form",
                description: "Enter a ballot ID",
                keyPrefixes: ["ballotLocator"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "found",
                label: "Found",
                description: "Ballot ID located",
                keyPrefixes: ["ballotLocator"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "not-found",
                label: "Not found",
                description: "Unknown ballot ID",
                keyPrefixes: ["ballotLocator"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "materials",
        label: "Support materials",
        variants: [
            {
                id: "default",
                label: "Materials list",
                description: "Support materials screen",
                keyPrefixes: ["materials"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "session",
        label: "Session & logout",
        variants: [
            {
                id: "timeout",
                label: "Session expiring",
                description: "Countdown warning dialog",
                keyPrefixes: ["header.session", "logout.modal"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "logout",
                label: "Logout confirm",
                description: "Logout confirmation dialog",
                keyPrefixes: ["logout"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
    {
        id: "error",
        label: "Error page",
        variants: [
            {
                id: "generic",
                label: "Unexpected error",
                description: "Generic error page",
                keyPrefixes: ["errors.page", "common"],
                extraKeys: sharedChromeKeys,
            },
            {
                id: "cert",
                label: "Certificate failed",
                description: "Certificate authentication error",
                keyPrefixes: ["errors.page", "common"],
                extraKeys: sharedChromeKeys,
            },
        ],
    },
]

export const getScene = (sceneId: string): SceneDefinition =>
    SCENES.find((scene) => scene.id === sceneId) ?? SCENES[0]

export const getVariant = (scene: SceneDefinition, variantId: string): SceneVariant =>
    scene.variants.find((variant) => variant.id === variantId) ?? scene.variants[0]
