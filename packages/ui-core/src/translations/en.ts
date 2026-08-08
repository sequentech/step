// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        language: "English",
        breadcrumbSteps: {
            select: "Select a Verifier",
            import: "Import Data",
            verify: "Verify",
            finish: "Finish",
        },
        electionEventBreadcrumbSteps: {
            created: "Created",
            keys: "Keys",
            publish: "Publish",
            started: "Started",
            ended: "Ended",
            results: "Results",
        },
        candidate: {
            moreInformationLink: "More information",
            writeInsPlaceholder: "Type write-in candidate here",
            blankVote: "Blank Vote",
            preferential: {
                position: "Position",
                none: "None",
                ordinals: {
                    first: "st",
                    second: "nd",
                    third: "rd",
                    other: "th",
                },
            },
        },
        homeScreen: {
            title: "Sequent Ballot Verifier",
            description1:
                "The ballot verifier is used when the voter chooses to audit the ballot in the voting booth. The verification should take 1-2 minutes.",
            description2:
                "The ballot verifier allows the voter to ensure that the encrypted ballot correctly captures the selections made in the voting booth. Allowing to perform this check is called cast-as-intended verifiability and prevents errors and malicious activity during ballot encryption.",
            descriptionMore: "Learn more",
            startButton: "Browse file",
            dragDropOption: "Or drag and drop it here",
            importErrorDescription:
                "There was a problem importing the auditable ballot. Did you choose  the right file?",
            importErrorMoreInfo: "More info",
            importErrorTitle: "Error",
            useSampleText: "Don't have an auditable ballot?",
            useSampleLink: "Use a sample auditable ballot",
        },
        confirmationScreen: {
            title: "Sequent Ballot Verifier",
            topDescription1:
                "Based on the information in the imported Auditable Ballot, we calculated that:",
            topDescription2: "If this is the Ballot ID shown in the Voting Booth:",
            bottomDescription1:
                "Your ballot was encrypted correctly. You can now close this window and return to the Voting Booth.",
            bottomDescription2:
                "If they don't match, click here to learn more about the potential reasons and what actions you can take.",
            ballotChoicesDescription: "And your ballot selections are:",
            helpAndFaq: "Help & FAQ",
            backButton: "Back",
            markedInvalid: "Ballot explicitly marked invalid",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content:
                    "The status panel gives you information about the  verifications performed.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Powered by <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write-in doesn't end on 0",
                writeInCharsExceeded_one: "Shorten your write-in by {{count}} character.",
                // `_many` is selected only in es/cat/fr, for exact multiples of a million. English never
                // selects it, but the other bundles are typed `TranslationType = typeof englishTranslation`,
                // so the key has to be declared here before they can carry it.
                writeInCharsExceeded_many: "Shorten your write-in by {{count}} characters.",
                writeInCharsExceeded_other: "Shorten your write-in by {{count}} characters.",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            implicit: {
                selectedMax_one: "Deselect {{count}} candidate.",
                selectedMax_many: "Deselect {{count}} candidates.",
                selectedMax_other: "Deselect {{count}} candidates.",
                selectedMin_one: "Select {{count}} more candidate.",
                selectedMin_many: "Select {{count}} more candidates.",
                selectedMin_other: "Select {{count}} more candidates.",
                maxSelectionsPerType_one: "Deselect {{count}} candidate from {{type}}.",
                maxSelectionsPerType_many: "Deselect {{count}} candidates from {{type}}.",
                maxSelectionsPerType_other: "Deselect {{count}} candidates from {{type}}.",
                underVote_one: "Select up to {{count}} more candidate.",
                underVote_many: "Select up to {{count}} more candidates.",
                underVote_other: "Select up to {{count}} more candidates.",
                overVoteDisabled_one:
                    "You have selected the maximum of {{count}} candidate. Deselect it to choose another.",
                overVoteDisabled_many:
                    "You have selected the maximum of {{count}} candidates. Deselect one to choose another.",
                overVoteDisabled_other:
                    "You have selected the maximum of {{count}} candidates. Deselect one to choose another.",
                blankVote: "You have not selected any candidate.",
                preferenceOrderWithGaps:
                    "Invalid vote! The order of preference has one or more gaps.",
                duplicatedPosition:
                    "Invalid vote! The same position was selected for two or more candidates.",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but the contest doesn't allow it",
                alert: "Selection marked will be considered invalid vote.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Invalid ballot configuration: the contest defines {{count}} explicitly invalid candidates, but only one is allowed.",
                multipleExplicitBlankCandidates:
                    "Invalid ballot configuration: the contest defines {{count}} explicit blank candidates, but only one is allowed.",
            },
        },
        ballotHash: "Your Ballot ID: {{ballotId}}",
        version: {
            header: "Version:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Logout",
            modal: {
                title: "Are you sure you want to logout?",
                content: "You are about to close this application. This action can not be undone. ",
                ok: "OK",
                close: "Close",
            },
        },
        stories: {
            openDialog: "Open Dialog",
        },
        dragNDrop: {
            firstLine: "Drag & drop files or",
            browse: "Browse",
            format: "Supported format: txt",
        },
        selectElection: {
            electionWebsite: "Ballot Website",
            countdown:
                "Election Begins in {{years}} years, {{months}} months, {{weeks}} weeks, {{days}} days, {{hours}} hours, {{minutes}} minutes, {{seconds}} seconds",
            openElection: "Open",
            closedElection: "Closed",
            voted: "Voted",
            notVoted: "Not voted",
            resultsButton: "Ballot Results",
            voteButton: "Click to Vote",
            openDate: "Open: ",
            closeDate: "Close: ",
            ballotLocator: "Locate your ballot",
        },
        header: {
            profile: "Profile",
            welcome: "Welcome,<br><span>{{name}}</span>",
            session: {
                title: "Your session is going to expire.",
                timeLeft: "You have {{time}} left to cast your vote.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutes and {{time}} seconds",
                timeLeftSeconds: "{{timeLeft}} seconds",
            },
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
