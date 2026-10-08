// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        language: "English",
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
        a11y: {
            closeDialog: "Close dialog",
            languageSelector: "Language: {{language}}",
            dismissMessage: "Dismiss message",
            ballotIdHelp: "About your Ballot ID",
            loading: "Loading",
            severity: {
                error: "Error",
                warning: "Warning",
                success: "Success",
                info: "Information",
            },
            selectList: "Select the whole list",
            preferenceLabel: "Preference",
            writeInFor: "Write-in candidate name",
        },
        accessibility: {
            button: "Accessibility",
            title: "Accessibility settings",
            description: "Change how this site looks on this device.",
            textSize: {
                label: "Text size",
                default: "Default",
                large: "Large",
                larger: "Larger",
            },
            contrast: {
                label: "Contrast",
                default: "Default",
                high: "High contrast",
            },
            textSpacing: {
                label: "Text spacing",
                default: "Default",
                wide: "Wide",
            },
            motion: {
                label: "Motion",
                default: "Default",
                reduced: "Reduced",
            },
            reset: "Reset settings",
            close: "Close",
            applied: "{{setting}}: {{value}}",
            resetDone: "Settings reset",
        },
        audioInstructions: {
            label: "Audio instructions",
            play: "Listen to the instructions",
            pause: "Pause the instructions",
            resume: "Resume the instructions",
            stop: "Stop the instructions",
            showTranscript: "Read the instructions",
            hideTranscript: "Hide the instructions",
            playing: "Playing the instructions",
            paused: "Instructions paused",
            stopped: "Instructions stopped",
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
            poweredBy: "Powered by <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write-in doesn't end on 0",
                writeInCharsExceeded:
                    "Write-in exceed by {{numCharsExceeded}} the maximum number of chars. Requires fixing.",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            implicit: {
                selectedMax:
                    "Overvote: Number of selected choices {{numSelected}} is more than the maximum {{max}}",
                selectedMin:
                    "Number of selected choices {{numSelected}} is less than the minimum {{min}}",
                maxSelectionsPerType:
                    "Number of selected choices {{numSelected}} for list {{type}} is more than the maximum {{max}}",
                underVote:
                    "You have not used all {{max}} of your selections in this contest. You may continue, or go back to make another selection.",
                overVoteDisabled:
                    "Maximum reached: You have selected the maximum of {{numSelected}} allowed. To change your selection, please deselect an option first.",
                blankVote: "Blank Vote: 0 choices selected",
                preferenceOrderWithGaps:
                    "Invalid vote! The order of preference has one or more gaps.",
                duplicatedPosition:
                    "Invalid vote! The same position was selected for two or more candidates.",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but question doesn't allow it",
                alert: "Selection marked will be considered invalid vote.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Invalid ballot configuration: the contest defines {{count}} explicitly invalid candidates, but only one is allowed.",
                multipleExplicitBlankCandidates:
                    "Invalid ballot configuration: the contest defines {{count}} explicit blank candidates, but only one is allowed.",
                invalidSlateConfiguration:
                    "Invalid ballot configuration: the slates are not valid ({{reason}}).",
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
            importError: "Could not import this file. Please try again.",
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
        // What an import or a validation found, by the id sequent-core gives
        // each complaint: `problems.messages.<area>.<complaint>.text`.
        problems: {
            noProblems: "Nothing to report. This would import.",
            errors_one: "{{count}} error",
            errors_other: "{{count}} errors",
            errorsExplained: "This will not import until every one of these is fixed.",
            warnings_one: "{{count}} warning",
            warnings_other: "{{count}} warnings",
            warningsExplained:
                "This will import. Each of these is something that is probably not what was meant.",
            warningsStrict: "Strict mode is on, so these stop the build.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Areas inside each other",
                        text: "Areas inside each other — area '{{area}}' is part of a loop of parents, which would hang the Admin Portal.",
                    },
                    "duplicate-name": {
                        lead: "Two areas share a name",
                        text: "Two areas share a name — '{{name}}' is both '{{first}}' and '{{second}}'. The voters CSV resolves an area by name, so voters would land in whichever the importer found first.",
                    },
                    "early-voting-unknown": {
                        lead: "Unknown early voting setting",
                        text: "Unknown early voting setting — '{{value}}' is not valid for an area. The platform accepts {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Area with an empty ballot",
                        text: "Area with an empty ballot — area '{{area}}' votes on no contest, neither its own nor one inherited from an area it sits inside, so its voters would see an empty ballot.",
                    },
                    "inside-itself": {
                        lead: "Area inside itself",
                        text: "Area inside itself — an area cannot be its own parent.",
                    },
                    "no-identifier": {
                        lead: "Area without an identifier",
                        text: "Area without an identifier — every area needs one.",
                    },
                    "no-name": {
                        lead: "Area without a name",
                        text: "Area without a name — the voters CSV identifies a voter's area by name, not by id, so an unnamed area is one no voter can be put in.",
                    },
                    "parent-missing": {
                        lead: "Parent area missing",
                        text: "Parent area missing — the area sits inside a parent that is not in this file.",
                    },
                    "parent-unknown": {
                        lead: "Parent area unknown",
                        text: "Parent area unknown — nothing has the identifier '{{parent}}'.",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "No elections",
                        text: "No elections — an election event needs at least one.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identifier used twice",
                        text: "Identifier used twice — {{id}} in {{kind}} is also used by {{previous}}.",
                    },
                    "event-no-encryption": {
                        lead: "No encryption protocol",
                        text: "No encryption protocol — the election event does not say how ballots are encrypted.",
                    },
                    "event-no-id": {
                        lead: "Event without an id",
                        text: "Event without an id — the election event in this file has no identifier.",
                    },
                    "no-areas": {
                        lead: "No areas",
                        text: "No areas — no voter can be given a ballot until the event has at least one area.",
                    },
                    "no-elections": {
                        lead: "No elections",
                        text: "No elections — an election event needs at least one election.",
                    },
                    "no-tenant": {
                        lead: "No tenant",
                        text: "No tenant — the file does not say which tenant it belongs to.",
                    },
                    "tenant-not-uuid": {
                        lead: "Tenant is not an identifier",
                        text: "Tenant is not an identifier — '{{value}}' is not a valid tenant id.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Candidate's contest missing",
                        text: "Candidate's contest missing — the candidate belongs to a contest that is not in this file.",
                    },
                    "no-contest": {
                        lead: "Candidate without a contest",
                        text: "Candidate without a contest — every candidate must belong to a contest.",
                    },
                    "picture-mismatch": {
                        lead: "Picture points elsewhere",
                        text: "Picture points elsewhere — the ballot shows '{{url}}', which does not name the document '{{document}}' beside it. After import the two would point at different files.",
                    },
                    "picture-not-shown": {
                        lead: "Picture never shown",
                        text: "Picture never shown — a candidate names a picture that no ballot entry points at, so it would be uploaded and never shown.",
                    },
                    "picture-unrecorded": {
                        lead: "Picture not recorded",
                        text: "Picture not recorded — a candidate's picture is on the ballot and nothing records which document it is, so it cannot be changed or removed on the platform afterwards.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Census column not declared",
                        text: "Census column not declared — {{message}}",
                    },
                    "no-area-column": {
                        lead: "Census names no areas",
                        text: "Census names no areas — this election has areas but every voter would get the default ballot. If the districting is meant to apply, the census needs an area column.",
                    },
                    "note": {
                        lead: "About the census",
                        text: "About the census — {{message}}",
                    },
                    "unreadable": {
                        lead: "Census unreadable",
                        text: "Census unreadable — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Census in the zip unreadable",
                        text: "Census in the zip unreadable — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Early voting switched off",
                        text: "Early voting switched off — {{areas}} allow early voting and the event does not open that channel, so the setting does nothing.",
                    },
                    "early-voting-no-area": {
                        lead: "Early voting with no area",
                        text: "Early voting with no area — early voting is open and no area allows it, so the early period would have no voters in it.",
                    },
                    "kiosk-client": {
                        lead: "Kiosk needs its own client",
                        text: "Kiosk needs its own client — kiosk voting needs an authentication client named after the ordinary one with '-kiosk' on the end, which this file does not create.",
                    },
                    "none-open": {
                        lead: "No way of voting",
                        text: "No way of voting — every voting channel is switched off, so nobody can vote.",
                    },
                    "telephone-elsewhere": {
                        lead: "Telephone voting set up later",
                        text: "Telephone voting set up later — telephone voting is configured on the event's IVR tab after import; none of it is in this file.",
                    },
                    "unknown": {
                        lead: "Unknown way of voting",
                        text: "Unknown way of voting — nothing reads '{{name}}'. The ways of voting the platform acts on are {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "No points of contact",
                        text: "No points of contact — on election day this is who gets called.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Unknown counting method",
                        text: "Unknown counting method — '{{value}}' is not a counting algorithm. The platform accepts {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Contest area unknown",
                        text: "Contest area unknown — nothing has the identifier '{{area}}'.",
                    },
                    "cap-invalid": {
                        lead: "Impossible selection limit",
                        text: "Impossible selection limit — {{cap}} is not a number of selections.",
                    },
                    "cap-never-applies": {
                        lead: "Selection limit never applies",
                        text: "Selection limit never applies — a limit of {{cap}} per type never applies in a contest where a voter may choose {{max}} in total.",
                    },
                    "chooses-more-than-available": {
                        lead: "More choices than candidates",
                        text: "More choices than candidates — a voter may choose {{max}} among {{available}} candidates.",
                    },
                    "chooses-more-than-offered": {
                        lead: "More choices than options",
                        text: "More choices than options — a voter may choose up to {{chosen}} but there are only {{offered}} to choose from.",
                    },
                    "columns-invalid": {
                        lead: "Impossible layout",
                        text: "Impossible layout — {{columns}} columns is not a layout.",
                    },
                    "columns-too-many": {
                        lead: "Too many columns",
                        text: "Too many columns — {{columns}} columns will be unreadable on a phone, which is how most voters vote.",
                    },
                    "count-missing": {
                        lead: "Contest missing a number",
                        text: "Contest missing a number — the contest needs {{field}}.",
                    },
                    "count-negative": {
                        lead: "Negative count",
                        text: "Negative count — {{field}} is {{value}}, and a count cannot be below zero.",
                    },
                    "election-missing": {
                        lead: "Contest's election missing",
                        text: "Contest's election missing — the contest belongs to an election that is not in this file.",
                    },
                    "elects-more-than-available": {
                        lead: "More seats than candidates",
                        text: "More seats than candidates — the contest elects {{winners}} from {{available}} candidates.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Elects more than allowed",
                        text: "Elects more than allowed — the contest elects {{winners}} but a voter may only choose {{chosen}}.",
                    },
                    "elects-more-than-standing": {
                        lead: "More seats than candidates",
                        text: "More seats than candidates — the contest elects {{winners}} from a field of {{standing}}.",
                    },
                    "elects-nobody": {
                        lead: "Elects nobody",
                        text: "Elects nobody — the contest has no winners.",
                    },
                    "max-votes-below-one": {
                        lead: "Nothing to vote for",
                        text: "Nothing to vote for — a voter may choose fewer than one candidate.",
                    },
                    "min-above-max": {
                        lead: "Minimum above maximum",
                        text: "Minimum above maximum — a voter must choose at least {{min}} but may choose at most {{max}}.",
                    },
                    "no-candidates": {
                        lead: "No candidates",
                        text: "No candidates — nobody is standing in this contest yet.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "No candidates",
                        text: "No candidates — the contest has no candidates, so nobody can vote in it.",
                    },
                    "on-no-ballot": {
                        lead: "Contest on no ballot",
                        text: "Contest on no ballot — no area's ballot includes this contest, so nobody can vote in it.",
                    },
                    "policy-not-text": {
                        lead: "Ballot rule is not text",
                        text: "Ballot rule is not text — {{key}} should be text, and is {{value}}.",
                    },
                    "policy-unknown": {
                        lead: "Unknown ballot rule",
                        text: "Unknown ballot rule — '{{value}}' is not a valid {{key}}. The platform accepts {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Ranked ballot counted without ranks",
                        text: "Ranked ballot counted without ranks — a preferential contest is counted by '{{algorithm}}', which ignores the rankings voters give.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Unknown tie-breaking rule",
                        text: "Unknown tie-breaking rule — '{{value}}' is not a tie-breaking policy. The platform accepts {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Plain ballot counted by rank",
                        text: "Plain ballot counted by rank — a non-preferential contest is counted by '{{algorithm}}', which needs ranked ballots.",
                    },
                    "voting-type-unknown": {
                        lead: "Unknown voting type",
                        text: "Unknown voting type — '{{value}}' is not a voting type. The platform accepts {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Write-in slots not allowed",
                        text: "Write-in slots not allowed — {{count}} write-in slots are on a contest that does not allow write-ins, which puts unnamed options on the ballot.",
                    },
                    "write-ins-no-slot": {
                        lead: "Write-ins with nowhere to write",
                        text: "Write-ins with nowhere to write — write-ins are allowed and the contest has no write-in slot, so a voter has nowhere to type a name.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "No importable archive",
                        text: "No importable archive — this zip has a plan but not the archive the Admin Portal imports, so the census and the files it names are not in it.",
                    },
                },
                design: {
                    "no-stable-key": {
                        lead: "Ballot design without a key",
                        text: "Ballot design without a key — {{kind}} {{id}} has no name or external id, so its ballot designs can't be recognized after an import.",
                    },
                    "unreadable-style": {
                        lead: "Ballot style unreadable",
                        text: "Ballot style unreadable — the platform's ballot style could not be read to compute its design digest: {{reason}}",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "Election and event disagree",
                        text: "Election and event disagree — {{channels}} differs from the event's, so the start controls for this election would not match.",
                    },
                    "grace-disallowed": {
                        lead: "Grace period switched off",
                        text: "Grace period switched off — {{seconds}} seconds of grace are set and no grace period is allowed, so voting closes on the deadline.",
                    },
                    "grace-negative": {
                        lead: "Negative grace period",
                        text: "Negative grace period — {{value}} seconds is not a length of time.",
                    },
                    "grace-zero": {
                        lead: "Grace period of no time",
                        text: "Grace period of no time — a grace period is allowed and lasts zero seconds, so there is none.",
                    },
                    "no-contests": {
                        lead: "No contests",
                        text: "No contests — nobody votes in this election.",
                    },
                    "revotes-negative": {
                        lead: "Impossible number of votes",
                        text: "Impossible number of votes — {{value}} is not a number of times a voter may vote.",
                    },
                    "setting-unknown": {
                        lead: "Unknown election setting",
                        text: "Unknown election setting — '{{value}}' is not a valid {{key}}. The platform accepts {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Spoiling with no second chance",
                        text: "Spoiling with no second chance — a voter may throw a cast ballot away and has no second attempt to replace it.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "No identifier",
                        text: "No identifier — every generated id is derived from it, so without one nothing can be built twice the same way.",
                    },
                    "no-name": {
                        lead: "No name",
                        text: "No name — voters see it above the ballot, and it becomes the login page's title.",
                    },
                    "setting-unknown": {
                        lead: "Unknown event setting",
                        text: "Unknown event setting — '{{value}}' is not a valid {{key}}. The platform accepts {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "Could not decrypt",
                        text: "Could not decrypt — the file is encrypted and did not open. Check the password.",
                    },
                    "checksum-mismatch": {
                        lead: "Not the expected file",
                        text: "Not the expected file — its SHA-256 does not match the one given, so it is not the file that was meant. Check the checksum or upload the file again.",
                    },
                    "duplicate-name": {
                        lead: "File name used twice",
                        text: "File name used twice — '{{file}}' names two different things. Files travel keyed by name, so one would silently become the other.",
                    },
                    "missing": {
                        lead: "File missing",
                        text: "File missing — '{{file}}' is named and nothing here holds it. An empty archive entry fails the import rather than losing a file.",
                    },
                    "not-a-bundle": {
                        lead: "Not an election event export",
                        text: "Not an election event export — the file was read but does not have the shape of one: {{reason}}",
                    },
                    "not-json": {
                        lead: "Not a readable file",
                        text: "Not a readable file — this is not an election event export the platform can read: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Archive unreadable",
                        text: "Archive unreadable — {{reason}}",
                    },
                    "unused": {
                        lead: "File not used",
                        text: "File not used — '{{file}}' was supplied and nothing names it, so it would travel in the delivery and be shown to nobody.",
                    },
                    "version-incompatible": {
                        lead: "Exported by another version",
                        text: "Exported by another version — the file comes from version {{found}}, which version {{current}} cannot import.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identifier used twice",
                        text: "Identifier used twice — '{{identifier}}' is already used by {{first}}. Identifiers are unique across the whole election event, so the second replaces the first instead of being added.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Language not spoken by phone",
                        text: "Language not spoken by phone — the telephone call speaks only English, French and Spanish, so callers are not offered {{languages}}.",
                    },
                    "missing-prompts": {
                        lead: "Call prompts missing",
                        text: "Call prompts missing — {{prompts}} have no words in '{{language}}', and the telephone system refuses every call until they do.",
                        lead_one: "Call prompt missing",
                        text_one:
                            "Call prompt missing — {{prompts}} has no words in '{{language}}', and the telephone system refuses every call until it does.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Permission labels in use",
                        text: "Permission labels in use — {{labels}}. Anything carrying a label is hidden from every administrator without it, so whoever imports this needs one of them on their own account or the Admin Portal will show them an empty list.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Default language not offered",
                        text: "Default language not offered — '{{chosen}}' is not among {{offered}}, so voters would get the first one instead.",
                    },
                    "detection-unknown": {
                        lead: "Unknown language detection",
                        text: "Unknown language detection — '{{policy}}' is not a policy the platform knows.",
                    },
                    "none": {
                        lead: "No languages",
                        text: "No languages — the ballot falls back to English, which is a safety net rather than a choice.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Ballot link to missing area",
                        text: "Ballot link to missing area — a contest is put on the ballot of an area that is not in this file.",
                    },
                    "contest-missing": {
                        lead: "Ballot link to missing contest",
                        text: "Ballot link to missing contest — an area's ballot lists a contest that is not in this file.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Two logos",
                        text: "Two logos — this plan carries both an uploaded logo ('{{file}}') and a link ('{{url}}'). The file is what ships; the link is ignored.",
                    },
                    "file-missing": {
                        lead: "Logo file missing",
                        text: "Logo file missing — '{{file}}' is named as the logo and was not supplied. Put the file beside the workbook under exactly that name.",
                    },
                    "no-bytes": {
                        lead: "Logo file empty",
                        text: "Logo file empty — '{{file}}' is named as the logo and carries no bytes. An empty archive entry fails the import rather than losing a picture.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Material with an empty document",
                        text: "Material with an empty document — it would import as a link to nothing. Leave the document out entirely for a material that has no file.",
                    },
                    "file-missing": {
                        lead: "Material file missing",
                        text: "Material file missing — '{{file}}' is named here and was not supplied. Put the file beside the workbook under exactly that name.",
                    },
                    "file-unused": {
                        lead: "Material file not used",
                        text: "Material file not used — '{{file}}' was supplied and no row names it, so it would be uploaded and shown to nobody.",
                    },
                    "no-identifier": {
                        lead: "Material without an identifier",
                        text: "Material without an identifier — its document's id is derived from it, so without one the file cannot be matched to the row.",
                    },
                    "tab-off": {
                        lead: "Materials tab switched off",
                        text: "Materials tab switched off — {{count}} support materials are in this file and the tab that shows them is off, so voters would never see them.",
                    },
                    "wrong-event": {
                        lead: "Material from another event",
                        text: "Material from another event — this support material belongs to a different election event.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "A repeat with no hour",
                        text: "A repeat with no hour — a message repeats every week but does not say at what time, so whoever sends it has to choose an hour nobody wrote down.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Passwords given twice",
                        text: "Passwords given twice — the census already has a password column, so generated passwords would be a second answer to the same question. Remove the column, or turn generating off.",
                    },
                    "no-characters": {
                        lead: "Passwords with no characters",
                        text: "Passwords with no characters — generated passwords need at least one kind of character to be made of.",
                    },
                    "no-seed": {
                        lead: "Passwords without a seed",
                        text: "Passwords without a seed — the seed is what makes a rebuild produce the same passwords rather than new ones.",
                    },
                },
                package: {
                    "already-imported": {
                        lead: "Already imported",
                        text: "Already imported — revision {{revision}} of this configuration was imported before; import a newer revision instead.",
                    },
                    "approval-invalid": {
                        lead: "Approval doesn't count",
                        text: "Approval doesn't count — the approval by {{name}} could not be verified: {{reason}}",
                    },
                    "approval-repeated": {
                        lead: "Same person approved twice",
                        text: "Same person approved twice — {{name}} approved more than once, and counts once.",
                    },
                    "approver-key-usage": {
                        lead: "Approver can't sign",
                        text: "Approver can't sign — an approver's certificate is not made for signing.",
                    },
                    "bad-signature": {
                        lead: "Signature doesn't match",
                        text: "Signature doesn't match — the package's signature does not verify, so it was changed after signing or signed by another key: {{reason}}",
                    },
                    "content-digest": {
                        lead: "Content digest doesn't match",
                        text: "Content digest doesn't match — the manifest says {{expected}} and its content hashes to {{actual}}.",
                    },
                    "duplicate-member": {
                        lead: "File name used twice",
                        text: "File name used twice — '{{file}}' appears twice in {{archive}}, so two readers could take different files.",
                    },
                    "file-changed": {
                        lead: "Changed after signing",
                        text: "Changed after signing — {{file}} has SHA-256 {{actual}}, and the manifest says {{expected}}. Nothing in the package was read.",
                    },
                    "file-extra": {
                        lead: "File not in the manifest",
                        text: "File not in the manifest — {{file}} is in the package but was not signed. Nothing in the package was read.",
                    },
                    "file-missing": {
                        lead: "Signed file missing",
                        text: "Signed file missing — {{file}} is in the manifest and not in the package. Nothing in the package was read.",
                    },
                    "invalid-time": {
                        lead: "Not a time",
                        text: "Not a time — '{{value}}' in the manifest is not a date and time.",
                    },
                    "member-too-large": {
                        lead: "File too large",
                        text: "File too large — '{{file}}' in {{archive}} expands to more than the {{limit}} bytes a file may.",
                    },
                    "nested-too-deep": {
                        lead: "Nested too deep",
                        text: "Nested too deep — '{{file}}' is inside more zips than the {{limit}} a file may be nested in.",
                    },
                    "no-importable": {
                        lead: "Nothing to import",
                        text: "Nothing to import — the package has no official_election_setup.zip, the archive the importer reads.",
                    },
                    "report-template-changed": {
                        lead: "Report template changed",
                        text: "Report template changed — the {{report}} report's template is not the approved one: its digest is {{actual}}, and the signed configuration says {{expected}}.",
                    },
                    "report-template-missing": {
                        lead: "Report template missing",
                        text: "Report template missing — the {{report}} report is drawn with template '{{template}}', which isn't in the configuration, so its design can't be signed.",
                    },
                    "report-unreadable": {
                        lead: "Report can't be signed",
                        text: "Report can't be signed — {{message}}",
                    },
                    "revoked-approver": {
                        lead: "Approver's certificate revoked",
                        text: "Approver's certificate revoked — an approver's certificate has been revoked, so the approval doesn't count.",
                    },
                    "revoked-signer": {
                        lead: "Signing key revoked",
                        text: "Signing key revoked — the key that signed this package has been revoked, and its packages are refused.",
                    },
                    "rollback": {
                        lead: "Not a newer revision",
                        text: "Not a newer revision — revision {{revision}} is not newer than revision {{last}}, the last one imported.",
                    },
                    "signed-in-the-future": {
                        lead: "Signed in the future",
                        text: "Signed in the future — the package says it was signed at {{at}}, and it is now {{now}}.",
                    },
                    "signer-key-usage": {
                        lead: "Signing key can't sign",
                        text: "Signing key can't sign — the certificate of the key that signed this package is not made for signing.",
                    },
                    "too-few-approvals": {
                        lead: "Too few approvals",
                        text: "Too few approvals — {{count}} valid approvals from different people, and {{required}} are needed.",
                    },
                    "too-large": {
                        lead: "Package too large",
                        text: "Package too large — it expands to more than the {{limit}} bytes a package may: '{{file}}' in {{archive}} is past them.",
                    },
                    "too-many-members": {
                        lead: "Too many files",
                        text: "Too many files — {{archive}} holds more files than the {{limit}} a package may hold.",
                    },
                    "unhashable-content": {
                        lead: "Content can't be hashed",
                        text: "Content can't be hashed — the configuration's content could not be written to be hashed: {{reason}}",
                    },
                    "unknown-format": {
                        lead: "Unknown manifest format",
                        text: "Unknown manifest format — the manifest is in format '{{format}}', which this version can't read.",
                    },
                    "unreadable-chain": {
                        lead: "Signer's certificates unreadable",
                        text: "Signer's certificates unreadable — the package's certificate chain could not be read: {{reason}}",
                    },
                    "unreadable-manifest": {
                        lead: "Manifest unreadable",
                        text: "Manifest unreadable — the package's manifest could not be read: {{reason}}",
                    },
                    "unreadable-revocation-list": {
                        lead: "Revocation list unreadable",
                        text: "Revocation list unreadable — a revocation list could not be read, so it can't be applied: {{reason}}",
                    },
                    "unreadable-trust": {
                        lead: "Trusted certificates unreadable",
                        text: "Trusted certificates unreadable — the {{setting}} setting could not be read: {{reason}}",
                    },
                    "unreadable-zip": {
                        lead: "Archive unreadable",
                        text: "Archive unreadable — {{archive}} could not be read as a zip: {{reason}}",
                    },
                    "unsigned": {
                        lead: "Package not signed",
                        text: "Package not signed — it has no {{missing}}, and this installation only imports signed packages.",
                    },
                    "untrusted-approver": {
                        lead: "Approver not trusted",
                        text: "Approver not trusted — an approver's certificate is not trusted: {{reason}}",
                    },
                    "untrusted-signer": {
                        lead: "Signer not trusted",
                        text: "Signer not trusted — the key that signed this package is not one this installation trusts: {{reason}}",
                    },
                    "unwritable-manifest": {
                        lead: "Manifest can't be written",
                        text: "Manifest can't be written — the manifest could not be written: {{reason}}",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Not an election plan",
                        text: "Not an election plan — the file could not be read as one: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Saved by a newer version",
                        text: "Saved by a newer version — {{saved}} against {{supported}}. Opening it here would silently drop whatever that version added.",
                    },
                    "unreadable": {
                        lead: "Plan unreadable",
                        text: "Plan unreadable — {{error}}",
                    },
                },
                reports: {
                    "duplicate": {
                        lead: "Report set twice",
                        text: "Report set twice — the {{report}} report is set more than once for the same election.",
                    },
                    "no-copies": {
                        lead: "No copies",
                        text: "No copies — the {{report}} report is set to print no copies. Set at least one.",
                    },
                    "unknown-election": {
                        lead: "Unknown election",
                        text: "Unknown election — the {{report}} report is about election '{{election}}', which this plan doesn't have.",
                    },
                    "unsupported-format": {
                        lead: "Format not available",
                        text: "Format not available — the {{report}} report can't be generated as {{format}}.",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Closes before it opens",
                        text: "Closes before it opens — voting would never be open.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Crosses a clock change",
                        text: "Crosses a clock change — the window is an hour longer or shorter than the times suggest.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Key ceremony too late",
                        text: "Key ceremony too late — the election key has to exist before a vote can be encrypted with it.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Tally ceremony too early",
                        text: "Tally ceremony too early — it would count votes that had not been cast yet.",
                    },
                    "window-incomplete": {
                        lead: "Voting window incomplete",
                        text: "Voting window incomplete — the period will have to be opened or closed by hand in the Admin Portal.",
                    },
                },
                signing: {
                    "duplicate-action": {
                        lead: "Two rules for one action",
                        text: "Two rules for one action — '{{action}}' has more than one signing rule. Keep one.",
                    },
                    "signatures-out-of-range": {
                        lead: "Signatures out of range",
                        text: "Signatures out of range — the rule for '{{action}}' must ask for between {{min}} and {{max}} signatures.",
                    },
                    "expiry-out-of-range": {
                        lead: "Expiry out of range",
                        text: "Expiry out of range — a '{{action}}' request must expire after 1 to 525,600 minutes (a year), or never.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Threshold too high",
                        text: "Threshold too high — {{threshold}} of {{trustees}} trustees are required, which cannot be met. The key would be generated and the result could never be decrypted.",
                    },
                    "one": {
                        lead: "Threshold of one",
                        text: "Threshold of one — any single trustee can open the tally alone, whatever the list says — the same guarantee as having one trustee, and none at all against that person.",
                    },
                    "zero": {
                        lead: "Threshold of zero",
                        text: "Threshold of zero — the tally could be opened without any trustee at all.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Not an email address",
                        text: "Not an email address — '{{email}}' does not look like one.",
                    },
                    "no-email": {
                        lead: "Trustee without an address",
                        text: "Trustee without an address — it is how they are invited to the key ceremony.",
                    },
                    "no-name": {
                        lead: "Trustee without a name",
                        text: "Trustee without a name — the importer resolves the key ceremony's members by name against the trustees already provisioned in the tenant, and an empty one silently becomes a member who does not exist.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "No trustees",
                        text: "No trustees — an election key needs at least two people to hold it. With one, that person can decrypt every ballot on their own, which is the guarantee the encryption is there to provide.",
                    },
                    "only-one": {
                        lead: "Only one trustee",
                        text: "Only one trustee — an election key needs at least two people to hold it. With one, that person can decrypt every ballot on their own, which is the guarantee the encryption is there to provide.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Voter's area unknown",
                        text: "Voter's area unknown — nothing is called '{{area}}'. Voters are matched to their area by name, so this voter would get no ballot. Copy the name from the area rather than retyping it.",
                    },
                    "duplicate-username": {
                        lead: "Username used twice",
                        text: "Username used twice — '{{username}}' is also row {{first}}. Two voters sharing a username become one account, and this one would replace the other without saying so.",
                    },
                    "no-area": {
                        lead: "Voter without an area",
                        text: "Voter without an area — the area decides which ballot a voter is handed, and the build refuses a census row without one.",
                    },
                    "no-username": {
                        lead: "Voter without a username",
                        text: "Voter without a username — it is what they sign in as and what their account is derived from.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Column given twice",
                        text: "Column given twice — two columns both set '{{column}}'. Keep only one of them.",
                    },
                    "unreadable-row": {
                        lead: "Row unreadable",
                        text: "Row unreadable — row {{row}} could not be read: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Vote weight column misspelled",
                        text: "Vote weight column misspelled — '{{column}}' is not recognised. The column is spelled exactly '{{expected}}', or every voter would be counted with weight 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "Vote weight not a number",
                        text: "Vote weight not a number — '{{value}}' on row {{row}} must be a whole number between 1 and {{max}}.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Vote weight out of range",
                        text: "Vote weight out of range — {{value}} on row {{row}} must be between {{min}} and {{max}}.",
                    },
                },
            },
        },
        timezones: {
            abbr: {
                "Asia/Manila": "PhST",
            },
            name: {},
            city: {},
            offset: "GMT{{sign}}{{hours}}:{{minutes}}",
            option: "({{offset}}) {{city}}",
            optionPrimary: "{{option}} · primary",
            optionDetail: "{{countries}} · {{name}}",
            dateTimeZone: "{{dateTime}} {{zone}}",
            myTime: "{{dateTime}} {{zone}} · my time",
            placeTime: "{{dateTime}} {{zone}} · {{place}}",
            voterDateTimeZone: "{{dateTime}} {{zoneName}}",
            onThisDevice: "On this device: {{dateTime}}",
            gap: "{{dateTime}} does not exist in {{city}} because clocks go forward. It will run at the time shown.",
            overlap: "{{dateTime}} happens twice in {{city}}. The first one is used.",
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
