// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        language: "Nederlands",
        welcome: "Hello <br/> <strong>World</strong>",
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
                position: "Positie",
                none: "Geen",
                ordinals: {
                    first: "e",
                    second: "e",
                    third: "e",
                    other: "e",
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
            ballotChoicesDescription: "And your ballot choices are:",
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
            poweredBy: "Aangedreven door <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write in doesn't end on 0",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            implicit: {
                selectedMax:
                    "Number of selected choices {{numSelected}} is more than the maximum {{max}}",
                selectedMin:
                    "Number of selected choices {{numSelected}} is less than the minimum {{min}}",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but question doesn't allow it",
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

export default dutchTranslation
