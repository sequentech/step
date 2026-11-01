// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        ballotPrivacy: {
            title: "Ballot box log",
            subtitle:
                "Every ballot cast in this election, listed under a pseudonym instead of the voter's name.",
            private: "Voters appear only as pseudonyms",
            signed: "Signed by the election authority and the ballot box",
            device: "Checked on your device",
            own: "Your ballot is in the ballot box",
            listedAs: "Listed as",
            check: "Check my ballot",
            copy: "Copy Ballot ID",
            ballotIdHelp: "About your Ballot ID",
            castTime: "Cast {{time}}",
            timesIn: "Times in {{zone}}",
            signedEntries: "Signed log entries ({{count}})",
            noMatchHelp:
                "Check for typing mistakes. The codes use the digits 0 to 9 and capital letters, without I, L, O and U.",
            helpSections: {
                pseudonyms: {
                    title: "Pseudonyms",
                    text: "Each voter is listed under a pseudonym, a code such as 7QKM-2HXD. The log never says who the voter is.",
                },
                signatures: {
                    title: "Signatures",
                    text: "The election authority signs which voter key each pseudonym uses. The ballot box signs each ballot it receives and each cast.",
                },
                checks: {
                    title: "Checks on your device",
                    text: "Open a ballot to check its signatures in this browser. Anyone can download its signed entries and check them again.",
                },
            },
            copied: "Ballot ID copied",
            copyFailed: "Could not copy the Ballot ID. Select the code to copy it.",
            search: "Find a Ballot ID or pseudonym",
            placeholder: "Ballot ID or pseudonym",
            clear: "Clear search",
            ballots: "{{count}} ballots",
            matches: "{{count}} of {{total}} ballots match",
            capped: "More than {{count}} of {{total}} ballots match; type more of the code",
            ballotId: "Ballot ID",
            pseudonym: "Pseudonym",
            castAt: "Cast",
            channel: "Channel",
            yours: "Yours",
            channels: {
                online: "Online",
                kiosk: "Kiosk",
                telephone: "Telephone",
            },
            previous: "Previous",
            next: "Next",
            rows: "Rows per page",
            noMatch: "No Ballot ID or pseudonym starts with {{prefix}}",
            noBallots: "No ballots have been cast yet.",
            invalidPrefix:
                "The codes use the digits 0 to 9 and capital letters, without I, L, O and U. Enter up to eight characters.",
            loadError: "The ballot box log could not be loaded. Please try again.",
            retry: "Try again",
            help: "About the ballot box log",
            helpText:
                "Each voter is listed under a pseudonym, a code such as 7QKM-2HXD. The log never says who the voter is. The election authority signs which voter key each pseudonym uses. Open a ballot to check its signatures in this browser.",
            detailTitle: "Ballot {{ballotId}}",
            checking: "Checking the signatures…",
            passed: "All four checks passed",
            verified: "Checked on this device with the published keys.",
            failed: "A check failed",
            failedHelp: "Do not rely on this entry.",
            pending: "Still being added to the log",
            pendingHelp: "The signed entries may appear after a ballot is cast. Check again later.",
            proofError: "The signed entries or published keys could not be loaded. Try again.",
            checkLabels: {
                binding: "Voter key registered",
                received: "Ballot received",
                ballotId: "Ballot ID",
                cast: "Ballot cast",
            },
            checkDescriptions: {
                binding: "The election authority signed which voter key this pseudonym uses.",
                received: "The ballot box received a ballot signed with that voter key.",
                ballotId: "The Ballot ID is computed from that signed ballot.",
                cast: "The same key cast the ballot, and the ballot box confirmed the cast.",
            },
            states: {
                passed: "Passed",
                failed: "Does not match.",
                pending: "Not in the log yet.",
            },
            neverPublished:
                "Never published: the voter's name, username, IP address, country or device.",
            download: "Download signed log entries",
            downloadAction: "Download",
            close: "Close",
            systemKey: "Election event log key",
            boxKey: "Ballot box key",
            userLogs:
                "A voter's own entries: sign-ins and messages. Their ballots are in the log under a pseudonym, so they never appear here; only a logged resolution links the two.",
            ballotSection: "Ballot and pseudonym",
            resolution: "Pseudonym resolution",
            resolutionHelp:
                "The answer went only to the person who asked. The voter is never named in this log.",
            reason: "Reason given",
            settings: {
                authority:
                    "The election event log key {{fingerprint}} signs which voter key each pseudonym uses.",
                loading: "Loading privacy permissions",
                loadError: "The current privacy permissions could not be loaded.",
                noRoles: "No role yet",
                title: "Privacy",
                identity: "Voter pseudonyms",
                on: "On: ballots, the log and its exports name a pseudonym for each voter, never the voter.",
                legacy: "This event uses the historical voter identity mode.",
                immutable:
                    "The identity mode is set when the event is created and cannot be changed.",
                network: "Network data",
                description:
                    "The IP address, country and browser of each ballot sent. They are never kept with ballots, voter records or log entries.",
                forensic: "Keep for forensic review",
                forensicHelp:
                    "Kept apart in the forensic store. Only people with Forensic Read see it, and every view is logged.",
                none: "Do not store",
                noneHelp:
                    "Nothing new is kept, and the dashboard's IP Addresses list is not shown. Previously retained forensic data follows its retention policy.",
                permissions: "Who can see or link it",
                permissionHelp:
                    "No preset role holds these permissions. Give them in Users and Roles to the few people who need them.",
                forensicPermission: "Forensic Read",
                forensicPermissionHelp: "Sees the network data in the forensic store",
                resolvePermission: "Resolve a pseudonym",
                resolvePermissionHelp:
                    "Links one pseudonym to its voter, with a reason. Each use is logged.",
                roles: "Manage in Users and Roles",
            },
            forensic: {
                allElections: "All permitted elections",
                exportError: "The forensic records could not be exported.",
                subtitle:
                    "Where ballots were sent from, by pseudonym. Only people with Forensic Read see this list, and each view is logged.",
                store: "Forensic store",
                pseudonyms: "Pseudonyms",
                first: "First ballot",
                last: "Last ballot",
                shared: "Shared",
                sharedHelp:
                    "Ballots of several pseudonyms came from this address, as from a hotel or office network.",
                more: "+{{count}} more",
                view: "View ballots",
                copy: "Copy IP address",
                detailTitle: "IP address {{ip}}",
                detailHelp:
                    "Opening this address was written to the election event’s log. Nobody is named here: each ballot is listed by its pseudonym.",
                error: "The forensic records could not be loaded.",
                empty: "No forensic ballot records for this address.",
            },
        },
        startup: {
            error: "The voting portal could not start. Please try again.",
            retry: "Try again",
        },
        contactChannel: {
            change: "Contact method",
            updated: "Your contact was changed. Codes and notices now go there.",
            cancelled: "Your contact was not changed.",
            failed: "We couldn't change how we contact you. Try again later or contact support.",
            close: "Close",
        },
        common: {
            goBack: "Back",
            showMore: "Show more",
            showLess: "Show less",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "This page lists the elections you can vote in. Use the Tab key to move from one election to the next. Each election has a button to start voting. Press Enter on that button to open its ballot.",
                "start":
                    "This page explains how to vote. First you choose your options, then you review them, and then you cast your ballot. Use the Tab key to reach the button that starts voting and press Enter. If a declaration is shown, tick its checkbox with the Space bar first.",
                "ballot":
                    "This is your ballot. Each question is a group of options, and the question says how many options you can choose. Use the Tab key to move between options and the Space bar to select or clear one. When you have finished, go to the button to continue, at the end of the page, and press Enter to review your choices. There is also a button to clear all your choices. Nothing is cast until you confirm on the review page.",
                "review":
                    "This page shows the choices you made. Check them carefully. To change something, use the button to edit your ballot. To cast your vote, go to the button to cast your ballot and press Enter. Your ballot ID is at the top of the page. You can copy it to find your ballot later.",
                "confirmation":
                    "Your vote has been cast. This page shows your ballot ID, which you can use to check that your ballot was recorded. You can copy it or print this page. When you are done, use the button at the end of the page to finish.",
                "audit":
                    "This page lets you audit your ballot instead of casting it. It shows the information that you can check with the ballot verifier. An audited ballot is not cast, so go back to your ballot afterwards to vote.",
                "ballot-locator":
                    "This page lets you check that a ballot was recorded. Type or paste a ballot ID in the text field and press Enter. The result appears below the field.",
                "support-materials":
                    "This page lists documents that help you vote. Use the Tab key to move between them and press Enter on a document's button to open it. Use the button to go back to return to the list of elections.",
            },
        },
        a11y: {
            skipToContent: "Skip to main content",
            helpAbout: "Help about {{topic}}",
            previewMaterial: "Preview {{title}}",
            ballotsTable: "Ballots",
            ballotLocatorTabs: "Ballot locator sections",
            ballotIdLabel: "Ballot ID",
            votingProgress: "Voting progress",
            stepOf: "Step {{current}} of {{total}}",
            selectUpTo_one: "Select up to {{count}} option",
            selectUpTo_many: "Select up to {{count}} options",
            selectUpTo_other: "Select up to {{count}} options",
            selectExactly_one: "Select {{count}} option",
            selectExactly_many: "Select {{count}} options",
            selectExactly_other: "Select {{count}} options",
            selectBetween: "Select between {{min}} and {{max}} options",
        },
        candidatesList: {
            collapseToggle: "Toggle list {{listTitle}}",
            showCandidates: "Show candidates",
            hideCandidates: "Hide candidates",
            selectedCandidates_one: "{{count}} candidate selected",
            // `_many` is selected only in es/cat/fr, for exact multiples of a million. English never
            // selects it, but the other bundles are typed `TranslationType = typeof englishTranslation`,
            // so the key has to be declared here before they can carry it.
            selectedCandidates_many: "{{count}} candidates selected",
            selectedCandidates_other: "{{count}} candidates selected",
            expandAll: "Expand all",
            collapseAll: "Collapse all",
        },
        breadcrumbSteps: {
            electionList: "Ballots",
            ballot: "Ballot",
            review: "Review",
            confirmation: "Confirm",
            audit: "Audit",
        },
        footer: {
            poweredBy: "Powered by <1></1>",
        },
        contest: {
            acclamation: {
                description:
                    "This contest was decided by acclamation. Its candidates are elected without a vote, so no option can be selected and no vote is recorded for it.",
            },
            paperBallot: {
                voteForNoMoreThan_one: "Vote for no more than {{count}}",
                voteForNoMoreThan_many: "Vote for no more than {{count}}",
                voteForNoMoreThan_other: "Vote for no more than {{count}}",
            },
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            clearButton: "Clear selections",
            ballotHelpDialog: {
                title: "About this screen",
                content:
                    "This screen shows the contests you are eligible to vote in. You can make your selection by activating the checkbox to the right of the Candidate/Answer. To reset your selections, click the “<b>Clear selections</b>” button; to move to the next step, click the “<b>Next</b>” button below.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Your vote is invalid or blank",
                content: "Some of your answers will make one or more contests invalid or blank",
                ok: "Review selections",
                continue: "Continue",
                cancel: "Cancel",
            },
            warningDialog: {
                title: "Review your ballot",
                content:
                    "Your ballot contains selections that may need your attention (such as selecting fewer options than allowed). Your ballot is valid and will be counted as submitted.",
                ok: "Back and review",
                continue: "Continue",
                cancel: "Cancel",
            },
            blankBallotDialog: {
                title: "You have not selected any candidates",
                content:
                    "You have not made any selections. Your ballot will be cast as a blank ballot, which is a valid, deliberate choice and will be counted as such.",
                continue: "Continue",
                cancel: "Cancel",
            },
        },
        startScreen: {
            startButton: "Start Voting",
            declineToVoteButton: "Decline to Vote",
            declineToVoteDialog: {
                title: "Confirm decline to vote",
                content:
                    "Are you sure you want to decline to vote?<br />You will go directly to review and your participation status will be saved as <b>Declined to vote</b>.",
                continue: "Decline to vote",
                cancel: "Cancel",
            },
            instructionsTitle: "How to vote",
            instructionsDescription: "Follow these steps to cast your ballot",
            step1Title: "1. Make your selections",
            step1Description:
                "Pick your preferred candidates and make your selections in each contest as it appears. You can change your ballot anytime before casting your vote",
            step2Title: "2. Review your selections",
            step2Description:
                "When you’re happy with your selections, we’ll securely encrypt your ballot and show you a final review. You’ll also get a unique tracker ID for reference",
            step3Title: "3. Cast your ballot",
            step3Description:
                "When you’re ready, cast your ballot so it’s officially recorded. Or choose to audit first to confirm it was correctly captured and encrypted",
        },
        reviewScreen: {
            castRecovery: {
                uncertain:
                    "The cast result has not been confirmed. You can retry the same request or check your Ballot ID before starting again.",
                checkBallot: "Check ballot",
            },
            acclamation: {
                title: "Decided by acclamation",
                helpDialog: {
                    title: "Information: Acclamation",
                    content:
                        "This screen shows what was decided by acclamation. Because no option could be selected, no ballot is cast and there is nothing to verify afterwards.",
                    ok: "OK",
                },
                description:
                    "Review what was decided by acclamation in this election. No ballot will be cast.",
                finishButton: "Finish",
            },
            title: "Review your ballot",
            description:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below, and to audit your ballot click the “<b>Audit Ballot</b>” button below.",
            descriptionNoAudit:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below.",
            backButton: "Edit ballot",
            castBallotButton: "Cast ballot",
            auditButton: "Audit ballot",
            copyBallotId: "Copy ballot ID",
            ballotIdCopied: "Ballot ID copied",
            ballotIdCopyError: "Could not copy ballot ID",
            reviewScreenHelpDialog: {
                title: "About the review screen",
                content: "This screen lets you review your selections before casting your ballot",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Your vote has not been cast",
                content:
                    "<p>This is your Ballot Tracker ID, but <b>your vote has not been cast yet</b>. If you try to track it now, nothing will appear.</p><p>We show the Tracker ID at this stage so you can audit the encrypted ballot before casting it.</p>",
                ok: "I understand my vote is not cast",
                cancel: "Cancel",
            },
            auditBallotHelpDialog: {
                title: "Would you like to audit your ballot",
                content:
                    "<p>Auditing your ballot will void it and restart the voting process. Continue only if you’re comfortable with the advanced steps. Otherwise, click <u>Cancel</u> to go back.</p>",
                ok: "Yes, discard my ballot to audit",
                cancel: "Cancel",
            },
            confirmCastVoteDialog: {
                title: "Are you sure you want to cast your vote?",
                content: "After you confirm, your vote will be cast.",
                ok: "Yes, I want to cast my vote",
                cancel: "Cancel",
            },
            confirmCastBlankBallotDialog: {
                title: "Are you sure you want to cast a blank ballot?",
                content:
                    "You have not selected any candidates. After you confirm, your ballot will be cast as blank.",
                ok: "Yes, cast my blank ballot",
                cancel: "Cancel",
            },
            unfilledContestsDialog: {
                title: "Some selections are unfilled",
                content:
                    "You may leave a selection blank or choose fewer options than allowed. Check the following before continuing.",
                selected: "{{selected}} of {{max}} selected",
                nothingSelected: "Nothing selected",
                ok: "Continue with these selections",
                cancel: "Review selections",
            },
            error: {
                NETWORK_ERROR:
                    "A network problem occurred. Please try again later or contact support",
                UNABLE_TO_FETCH_DATA:
                    "There was a problem fetching the data. Please try again later or contact support for assistance.",
                LOAD_ELECTION_EVENT: "Unable to load the election event. Please try again later",
                CAST_VOTE:
                    "An unknown error occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_AreaNotFound:
                    "There was an error while casting the vote: Area not found. Please try again later or contact support for assistance.",
                CAST_VOTE_CheckStatusFailed:
                    "This election does not allow casting a vote. It may be closed, archived, or outside the allowed voting period",
                CAST_VOTE_InternalServerError:
                    "An internal error occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_QueueError:
                    "There was a problem processing your vote. Please try again later or contact support",
                CAST_VOTE_Unauthorized:
                    "You are not authorized to cast a vote. Please contact support.",
                CAST_VOTE_ElectionEventNotFound:
                    "The election event could not be found. Please try again later or contact support.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Your voting record could not be found. Please contact support",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "An error occurred while checking your voting status. Please try again later or contact support",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Failed to verify your credentials. Please try again later or contact support",
                CAST_VOTE_GetAreaIdFailed:
                    "An error occurred while verifying your voting area. Please try again later or contact support",
                CAST_VOTE_GetTransactionFailed:
                    "An error occurred while processing your vote. Please try again later or contact support",
                CAST_VOTE_DeserializeBallotFailed:
                    "An error occurred while loading your ballot. Please try again later or contact support",
                CAST_VOTE_DeserializeContestsFailed:
                    "An error occurred while loading your selections. Please try again later or contact support.",
                CAST_VOTE_PokValidationFailed:
                    "Failed to validate your vote. Please try again later or contact support",
                CAST_VOTE_UuidParseFailed:
                    "An error occurred while processing your request. Please try again later or contact support.",
                CAST_VOTE_unexpected:
                    "An unknown error occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_timeout:
                    "A timeout occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "You have exceeded the revote limit. Please contact support.",
                CAST_VOTE_CheckRevotesFailed:
                    "You have exceeded the allowed number of revotes. Please contact support",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "You have already voted in another area. Please contact support",
                CAST_VOTE_UnknownError:
                    "An unknown error occurred while casting your vote. Please try again later or contact support",
                NO_BALLOT_SELECTION:
                    "The selection state for this election is missing. Please check your selections or contact support",
                NO_BALLOT_STYLE: "This ballot is not available. Please contact support",
                NO_AUDITABLE_BALLOT:
                    "There is no ballot available for audit. Please contact support",
                INCONSISTENT_HASH:
                    "There was an error related to the ballot hashing process. BallotId: {{ballotId}} is not consistent with auditable Ballot Hash: {{auditableBallotHash}}. Please report this issue to support.",
                ELECTION_EVENT_NOT_OPEN: "This election is closed. You can no longer vote",
                PARSE_ERROR:
                    "There was an error parsing the ballot. Please try again later or contact support for assistance.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "There was an error deserializing the auditable ballot. Please try again later or contact support for assistance.",
                DESERIALIZE_HASHABLE_ERROR:
                    "There was an error deserializing the hashable ballot. Please try again later or contact support for assistance.",
                CONVERT_ERROR:
                    "There was an error converting the ballot. Please try again later or contact support for assistance.",
                SERIALIZE_ERROR:
                    "There was an error serializing the ballot. Please try again later or contact support for assistance.",
                UNKNOWN_ERROR: "An error occurred. Please try again later or contact support",
                REAUTH_FAILED:
                    "Login failed. Your username or password may be incorrect. Please try again or contact support",
                SESSION_EXPIRED: "Your session has expired. Please start again",
                CAST_VOTE_BallotIdMismatch: "The ballot id does not match with the cast vote.",
                SESSION_STORAGE_ERROR:
                    "Session storage is not available. Please try again or contact support.",
                PARSE_BALLOT_DATA_ERROR:
                    "There was an error parsing the ballot data. Please try again later or contact support for assistance.",
                NOT_VALID_BALLOT_DATA_ERROR: "The ballot data is invalid. Please contact support",
                FETCH_DATA_TIMEOUT_ERROR:
                    "A timeout occurred while loading the data. Please try again",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error converting to hashable ballot. Please try again later or contact support for assistance.",
                INTERNAL_ERROR:
                    "There was an internal error while casting the vote. Please try again later or contact support for assistance.",
            },
            declineToVote: "Decline to vote",
            blankBallot: "Blank ballot",
        },
        confirmationScreen: {
            acclamation: {
                title: "Decided by acclamation",
                description:
                    "Every contest in this election was decided by acclamation, so no ballot was cast and there is no ballot ID to track.",
                helpDialog: {
                    title: "Information: Acclamation",
                    content:
                        "Every contest in this election was decided by acclamation: its candidates are elected without a vote. Because no ballot was cast, there is no ballot ID, receipt or QR code to verify.",
                    ok: "OK",
                },
            },
            title: "Your vote has been cast",
            description:
                "Your ballot was cast successfully. Use the code below to verify that it was counted",
            blankBallot: {
                description: "Your ballot was cast blank, which is a valid, deliberate choice.",
            },
            ballotId: "Ballot ID",
            printButton: "Print",
            finishButton: "Finish",
            remainingElectionsError:
                "We couldn’t check whether you have more elections to vote in. Please retry.",
            retryButton: "Retry",
            verifyCastTitle: "Verify that your ballot was cast",
            verifyCastDescription:
                "You can verify your ballot was cast correctly at any time using the QR code below",
            confirmationHelpDialog: {
                title: "About the confirmation screen",
                content:
                    "This screen confirms that your vote was successfully cast. The information here allows you to verify that your ballot was stored in the ballot box, both during the voting period and after it has closed",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Printing ballot",
                content: "Printing is disabled in demo mode",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "Ballot Id",
                content: "Code use is disabled in demo mode",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "About the Ballot ID",
                content:
                    "The Ballot ID is a unique code that lets you find your ballot in the ballot box. It does not contain any information about your selections.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "About the Ballot ID",
                content:
                    "The Ballot ID is a unique code that lets you find your ballot in the ballot box. It does not contain any information about your selections.",
                ok: "OK",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "An error occurred. Please try again",
                ok: "OK",
            },
            demoQRText: "Ballot tracker is disabled in demo mode",
        },
        auditScreen: {
            confirmingAudit: "Confirming that this ballot cannot be cast…",
            auditUnavailable:
                "The ballot could not be audited. Its contents have not been revealed. Please try again.",
            printButton: "Print",
            restartButton: "Start Voting",
            title: "Check your ballot",
            description: "To check your ballot, follow the steps below:",
            step1Title: "1. Save the following details:",
            step1Description:
                "your <b>Ballot ID</b> at the top of the screen and your encrypted ballot shown below",
            step1HelpDialog: {
                title: "Copy ballot code",
                content:
                    "You can download or copy your ballot code to verify that it correctly reflects your selections.",
                ok: "OK",
            },
            downloadButton: "Download",
            step2Title: "2. Check your ballot",
            step2Description:
                "Click <VerifierLink>Check your ballot code</VerifierLink>. It will open in a new tab",
            step2HelpDialog: {
                title: "How to check your ballot code",
                content:
                    "To check your ballot code, follow the steps in the how-to guide. This includes downloading a desktop application that lets you verify your ballot independently from the website.",
                ok: "OK",
            },
            bottomWarning:
                "For security reasons, when you audit your ballot, it needs to be spoiled. To continue with the voting process, you need to click ‘<b>Start Voting</b>’ below.",
        },
        electionSelectionScreen: {
            title: "Ballot list",
            description: "Select the ballot you want to vote on",
            chooserHelpDialog: {
                title: "About the Ballot list",
                content:
                    "Welcome to the Voting Booth. This screen shows the ballots you can vote on. Ballots displayed in this list can be open to voting, scheduled, or closed. You will be able to access the ballot only if the voting period is open.",
                ok: "OK",
            },
            noResults: "No ballots for now.",
            resultsButton: "View results",
            demoDialog: {
                title: "Demo voting booth",
                content:
                    "You are entering a demo voting booth. <strong>Your vote will not be cast.</strong> This booth is for demonstration only.",
                ok: "I understand that my vote will not be cast",
            },
            errors: {
                noVotingArea:
                    "You are not listed as a voter in this election. Please contact support.",
                networkError:
                    "A network problem occurred. Please try again later or contact support",
                unableToFetchData:
                    "There was a problem fetching the data. Please try again later or contact support for assistance.",
                noElectionEvent:
                    "Election event doesn’t exist. Please try again later or contact support for assistance.",
                ballotStylesEmlError:
                    "There was an error with the publish ballot style. Please try again later or contact support for assistance.",
                obtainingElectionFromID:
                    "There was an error obtaining elections associated with the following election IDs: {{electionIds}}. Please try again later or contact support for assistance.",
            },
            alerts: {
                noElections:
                    "There are no elections you can vote for. This could be because the area doesn’t have any contest associated. Please try again later or contact support for assistance.",
                electionEventNotPublished:
                    "The election event hasn’t been published yet. Please try again later or contact support for assistance.",
            },
            materialsGate: {
                instructions:
                    "You must read the <MaterialsLink>{{materialsTitle}}</MaterialsLink> before you can vote.",
            },
            votingClosedAt: "Voting closed on {{close}} ({{localClose}}).",
            votingClosedOn: "Voting closed on {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write-in doesn't end on 0",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but the contest doesn't allow it",
                alert: "This selection will be counted as an invalid vote",
            },
            page: {
                oopsWithStatus: "Oops! {{status}}",
                oopsWithoutStatus: "Oops! Unexpected Error",
                somethingWrong: "Something went wrong.",
                certAuthFailedTitle: "Certificate Authentication Failed",
                certAuthFailedMessage:
                    "Your certificate could not be verified. Please check that you are using a valid voter certificate and try again.",
                invalidLoginHintParametersTitle: "Invalid voting link",
                invalidLoginHintParametersMessage:
                    "This voting link contains invalid login information. Request a new link and try again.",
            },
        },
        materials: {
            common: {
                label: "Support Materials",
                back: "Back to ballot list",
                close: "Close",
                preview: "Preview",
                download: "Download",
            },
            mandatory: {
                checkboxLabel: "I have read the Support Materials",
                continueButton: "Continue",
                error: "There was a problem recording your acknowledgment. Please try again.",
            },
        },
        slates: {
            title: "Slates",
            description:
                "A slate is a group of candidates running together. Each candidate shows the slate they belong to.",
            independent: "Independent",
            contestMembers: "{{slate}} candidates for {{contest}}",
            noCandidate: "No candidate",
            coverage: {
                full: "Full slate",
                singleContest: "{{contest}} only",
                partial: "Partial slate",
                candidates_one: "{{count}} candidate",
                candidates_other: "{{count}} candidates",
                offices_one: "{{count}} office",
                offices_other: "{{count}} offices",
            },
            candidateList: {
                show: "Show candidates",
                hide: "Hide candidates",
            },
            tabs: {
                label: "Ways to fill in your ballot",
                slates: "Choose a slate",
                candidates: "Individual candidates",
            },
            review: {
                title: "Your selections",
                total: "Candidates selected: {{selected}} of {{seats}}",
                slate: "{{slate}}: {{status}}",
                independent: "Independent candidates selected: {{count}}",
                note: "Your vote is recorded for each selected candidate. A slate is not a vote of its own.",
                contestCount: "{{selected}} of {{max}} selected",
                edit: "Edit",
                editLabel: "Edit {{contest}}",
            },
            selection: {
                all: "All {{total}} selected",
                mixed: "Mixed · {{selected}} of {{total}} selected",
                partly: "Partly selected · {{selected}} of {{total}}",
                selected: "Selected",
                edit: "Edit selections",
            },
            apply: {
                button: "Choose this slate",
                buttonLabel: "Choose slate {{slate}}",
                chosen: "{{slate}} chosen. Candidates selected: {{candidates}}. Contests: {{contests}}.",
                overMaximum:
                    "{{slate}} cannot be chosen: it has {{candidates}} candidates for {{contest}}, which allows {{max}}. You can still choose candidates individually.",
                unavailable:
                    "{{slate}} cannot be chosen on this ballot. You can still choose candidates individually.",
                replaceDialog: {
                    title: "Replace your current choices?",
                    content:
                        "Choosing {{slate}} replaces your choices in the contests below. Your other choices stay as they are.",
                    removed: "Removed:",
                    added: "Selected instead:",
                    ok: "Replace choices",
                    cancel: "Keep my choices",
                },
            },
        },
        ballotLocator: {
            title: "Find your Ballot",
            titleResult: "Your Ballot Lookup Results",
            description: "Confirm your ballot was cast correctly",
            locate: "Find your Ballot",
            locateAgain: "Find another Ballot",
            found: "Your ballot ID {{ballotId}} has been found",
            notFound: "Your ballot ID {{ballotId}} was not found",
            ambiguous:
                "More than one of your ballots matches {{ballotId}}. Use the full ballot ID.",
            contentDesc: "This is your Ballot content: ",
            wrongFormatBallotId: "Invalid Ballot ID format",
            ballotIdNotFoundAtFilter:
                "Not found, check that your Ballot ID is correct and belongs to this user.",
            filterByBallotId: "Filter by Ballot ID",
            totalBallots: "Total Ballots: {{total}}",
            checksAvailableUntil: "You can check your ballot until {{date}}.",
            checksEnded: "Checks ended on {{date}}.",
            castAt: "Cast on {{date}}",
            steps: {
                lookup: "Find your Ballot",
                result: "Result",
            },
            titleHelpDialog: {
                title: "About the Ballot Finder Screen",
                content:
                    "This screen allows you to find your vote by using the Ballot ID to retrieve it. This procedure enables you to check that your ballot was correctly cast and that the recorded ballot matches the encrypted ballot you sent.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Ballot Locator",
            },
            column: {
                statement_kind: "Statement kind",
                statement_timestamp: "Statement Timestamp",
                ballot_id: "Ballot ID",
            },
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
