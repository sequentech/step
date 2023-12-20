// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        breadcrumbSteps: {
            electionList: "Election List",
            ballot: "Ballot",
            review: "Review",
            confirmation: "Confirmation",
            audit: "Audit",
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            ballotHelpDialog: {
                title: "Information: Ballot screen",
                content:
                    "This screen shows the contest you are elegible to vote. You can make your section by activate the checkbox on the Candidate/Answer right. To reset your selections, click “<b>Clear selection</b>” button, to move to next step, click “<b>Next</b>” button bellow.",
                ok: "OK",
            },
        },
        startScreen: {
            startButton: "Start Voting",
            instructionsTitle: "Instructions",
            instructionsDescription: "You need to follow these steps to cast your ballot:",
            step1Title: "1. Select your options",
            step1Description:
                "Answer to the election questions one by one as they are shown. This way you will configure your preferences in your ballot.",
            step2Title: "2. Review your ballot",
            step2Description:
                "Once you have chosen your preferences, we will proceed to encrypt them and you'll be shown the ballot's tracker id. You'll also be shown a summary with the content of your ballot for review.",
            step3Title: "3. Cast your ballot",
            step3Description:
                "You can cast it so that it's properly registered. Alternatively, you can audit that your ballot was correctly encrypted.",
        },
        reviewScreen: {
            title: "Review your ballot",
            description:
                "To make changes in your selections, click “<b>Change selection</b>” button, to confirm your selections, click “<b>Submit Ballot</b>” button bellow, and to audit your ballot click the “<b>Audit the Ballot</b>” button bellow. Please note than once you submit your ballot, you have voted and you will not be issued another ballot for this election.",
            backButton: "Edit ballot",
            castBallotButton: "Cast your ballot",
            auditButton: "Audit ballot",
            reviewScreenHelpDialog: {
                title: "Information: Review Screen",
                content:
                    "This screen allows you to review your selections before casting your ballot.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Vote has not been cast",
                content:
                    "<p>This is your Ballot Tracker ID, but <b>your vote has not been cast yet</b>. If you try to track the ballot, you will not find it.</p><p>The reason we show the Ballot Tracker ID at this stage is to allow you to audit the correctness of the encrypted ballot before casting it.</p>",
                ok: "I accept my vote has NOT been cast",
                cancel: "Cancel",
            },
            auditBallotHelpDialog: {
                title: "Do you want to audit the ballot?",
                content:
                    "<p>Auditing the ballot will spoil it and you will need to start the process of voting again if you want to cast your vote. The ballot audit process allows you to verify it's correctly encoded. Doing this process requires you to have important technical knowledge, so we do not recommend it if you do not know what you are doing.</p><p><b>If you just want to cast your ballot, click <u>Cancel</u> to go back to the review ballot screen.</b></p>",
                ok: "Yes, I want to DISCARD my ballot to audit it",
                cancel: "Cancel",
            },
        },
        logsScreen: {
            noPermissions: "You don't have permission to access logs.",
            title: "Logs",
            subtitle: "General logs of the main and IAM databases",
            column: {
                id: "Id",
                statement: "Statement",
            },
            main: {
                title: "Main Database Logs",
            },
            iam: {
                title: "IAM Database Logs",
            },
        },
        confirmationScreen: {
            title: "Your vote has been cast",
            description:
                "The confirmation code bellow verifies that <b>your ballot has been cast successfully</b>. You can use this code to verify that your ballot has been counted.",
            ballotId: "Ballot ID",
            printButton: "Print",
            finishButton: "Finish",
            verifyCastTitle: "Verify that your ballot has been cast",
            verifyCastDescription:
                "You can verify your ballot has been cast correctly at any moment using the following QR code:",
            confirmationHelpDialog: {
                title: "Information: Confirmation Screen",
                content:
                    "This screen shows that your vote was successfully cast. The information provided on this page allows you to verify that the ballot has been stored in ballot box , this process can be executed at any time during voting period and after the election has been closed.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Information: Ballot ID",
                content:
                    "The Ballot ID is a code that allows you to find your ballot in the ballot box, this ID is unique and doesn't contain information about your selections.",
                ok: "OK",
            },
        },
        auditScreen: {
            printButton: "Print",
            restartButton: "Start Voting",
            title: "Audit your Ballot",
            description: "To verify your ballot you will need. to follow the bellow steps:",
            step1Title: "1. Download or copy the following information",
            step1Description:
                "Your <b>Ballot ID</b> that appears at the top of the screen and your encrypted ballot below:",
            step1HelpDialog: {
                title: "Copy the Encrypted Ballot",
                content:
                    "You can download or copy your encrypted ballot to audit the ballot and verify the encrypted content contains your selections.",
                ok: "OK",
            },
            downloadButton: "Download",
            step2Title: "2. Follow the steps on this tutorial",
            step2Description:
                '(<a href="https://github.com/sequentech/new-ballot-verifier/blob/main/README.md">click here</a>, a new tab will open in your browser)',
            step2HelpDialog: {
                title: "Audit ballot tutorial",
                content:
                    "To audit your ballot you will need to follow the steps shown in the tutorial, this includes the download of a desktop application used to verify the encrypted ballot independently from the website.",
                ok: "OK",
            },
            bottomWarning:
                "For security reason, when you audit your ballot, it need to be spoiled. To continue with the voting process, you need to click ‘<b>Start Voting</b>’ bellow.",
        },
        electionSelectionScreen: {
            title: "Election list",
            description: "Select the election you want to vote",
            chooserHelpDialog: {
                title: "Information: Election List",
                content:
                    'Welcome to the Voting Booth, this screen shows the list of elections you can cast a ballot. Elections displayed in this list can be open to voting, scheduled, or closed. You will be able to access the ballot only if the voting period is open. In the case an election is closed and your election administrator has published the result you will see an "Election Result" button that will take you to the public result page.',
                ok: "OK",
            },
        },
        areas: {
            common: {
                title: "Areas",
                subTitle: "Area configuration.",
            },
            createAreaSuccess: "Area created",
            createAreaError: "Could not create Area",
            sequent_backend_area_contest: "Contests",
        },
        electionTypeScreen: {
            common: {
                title: "Election Type",
                subtitle: "Election type configuration",
                onlineVoting: "Online Voting",
                kioskVoting: "Kiosk Voting",
                settingTitle: "Settings",
                settingSubtitle: "General Configuration",
                sms: "SMS",
                mail: "Mails",
                spanish: "Spanish",
                english: "English",
                createNew: "Create Election Type",
                emptyHeader: "No Election Types yet.",
                emptyBody: "Do you want to create one?",
            },
            create: {
                title: "Create Election Type",
            },
            edit: {
                title: "Edit Election Type",
            },
            tabs: {
                votingChannels: "VOTING CHANELS",
                electionTypes: "ELECTION TYPES",
                communications: "COMMUNICATION",
                languages: "LANGUAGES",
            },
        },
        dashboard: {
            voteByDay: "Vote by day",
            voteByChannels: "Vote by channels",
        },
        electionEventScreen: {
            common: {
                subtitle: "Election event configuration.",
            },
            edit: {
                general: "General",
                dates: "Dates",
                language: "Language",
                allowed: "Voting Channels Allowed",
            },
            field: {
                name: "Name",
                alias: "Alias",
                description: "Description",
                startDateTime: "Start Date and Time",
                endDateTime: "End Date and Time",
                language: "Language",
                votingChannels: "Voting Channels",
            },
            error: {
                endDate: "End date must be after start date",
                noResult: "No Election Event yet",
            },
            voters: {
                title: "Voters",
            },
            createElectionEventSuccess: "Election Event created",
            createElectionEventError: "Error creating election event",
            stats: {
                elegibleVoters: "Elegible voters",
                elections: "Elections",
                contests: "Contests",
                areas: "Areas",
                sentEmails: "Emails sent",
                sentSMS: "SMS sent",
                calendar: {
                    title: "Calendar",
                    scheduled: "Scheduled",
                },
            },
            keys: {
                createNew: "Create Keys Ceremony",
                emptyHeader: "No Keys Ceremony yet.",
                statusLabel: "Status",
                waitingKeys: "Waiting for Keys Generation..",
                started: "Started at",
                breadCrumbs: {
                    configure: "Configure",
                    ceremony: "Ceremony",
                    created: "Finished",
                    start: "Start",
                    status: "Status",
                    download: "Download",
                    check: "Check",
                    success: "Finished",
                },
                notify: {
                    participateNow:
                        "You have been invited to participate in a Keys ceremony. Please <1>click on the ceremony's Key Action</1> to participate.",
                },
            },
            tabs: {
                dashboard: "Dashboard",
                data: "Data",
                voters: "Voters",
                areas: "Areas",
                keys: "Keys",
                tally: "Tally",
                publish: "Publish",
                logs: "Logs",
            },
            tally: {
                emptyHeader: "No Tally yet.",
                title: "Election Event Tally",
                elections: "Elections",
                electionNumber: "Number Elections",
                trustees: "Trustees",
                status: "Status",
                create: {
                    title: "Create Tally",
                    subtitle: "Create a new Tally for this Election Event",
                    createButton: "Start Tally Ceremony",
                    error: {
                        create: "Error creating Tally",
                    },
                    success: "Tally created",
                },
                logs: {
                    noLogs: "No logs available",
                },
            },
            import: {
                eetitle: "Import Election Event",
                eesubtitle: "Import election event data",
                title: "Import Voters",
                subtitle: "Import voters data",
                voters: "Voters",
                elections: "Elections",
                areas: "Areas",
                sha: "Integrity (SHA 256)",
                cancel: "Cancel",
                import: "Import",
            },
        },
        electionScreen: {
            common: {
                title: "Election",
                subtitle: "Election configuration.",
                fileLoaded: "File loaded",
            },
            edit: {
                general: "General",
                dates: "Dates",
                language: "Language",
                allowed: "Voting Channels Allowed",
                default: "Default",
                receipts: "Receipts",
                image: "Image",
                advanced: "Advanced Configuration",
            },
            field: {
                name: "Name",
                language: "Language",
                votingChannels: "Voting Channels",
                startDateTime: "Start Date and Time",
                endDateTime: "End Date and Time",
                alias: "Alias",
                description: "Description",
            },
            error: {
                endDate: "End date must be after start date",
                fileError: "Error uploading file",
                fileLoaded: "File loaded",
            },
            createElectionEventSuccess: "Election Event created",
            createElectionEventError: "Error creating election event",
            tabs: {
                dashboard: "Dashboard",
                data: "Data",
                voters: "Voters",
                publish: "Publish",
                logs: "Logs",
            },
        },
        tenantScreen: {
            common: {
                title: "Client",
            },
            new: {
                subtitle: "Create Client",
            },
            createSuccess: "Customer created",
            createError: "Error creating customer",
        },
        usersAndRolesScreen: {
            common: {
                title: "Users and Roles",
                subtitle: "General configuration",
                mobileNumber: "Mobile",
            },
            users: {
                title: "Users",
                subtitle: "View and edit user data",
                edit: {
                    title: "User Data",
                    subtitle: "View and edit user",
                },
                create: {
                    title: "User",
                    subtitle: "Create user",
                },
                fields: {
                    username: "Username",
                    first_name: "First Name",
                    last_name: "Last Name",
                    email: "Email",
                    enabled: "Enabled",
                    emailVerified: "Email Verified",
                    groups: "Groups",
                    attributes: "Attributes",
                    area: "Area",
                    password: "Password",
                    repeatPassword: "Repeat Password",
                    passwordMismatch: "Passwords must match",
                    passwordLengthValidate: "Password must be at least 8 characters long",
                    passwordUppercaseValidate:
                        "Password must contain at least one uppercase letter",
                    passwordLowercaseValidate:
                        "Password must contain at least one lowercase letter",
                    passwordDigitValidate: "Password must contain at least one digit",
                    passwordSpecialCharValidate:
                        "Password must contain at least one special character",
                },
                delete: {
                    body: "Are you sure you want to delete this user?",
                    bulkBody: "Are you sure you want to delete the selected users?",
                },
                notifications: {
                    deleteError: "Error deleting user",
                    deleteSuccess: "User deleted",
                },
            },
            voters: {
                title: "Voters",
                subtitle: "View and edit voter data",
                create: {
                    title: "Voter",
                    subtitle: "Create Voter",
                },
                emptyHeader: "No voters yet.",
                askCreate: "Do you want to create one?",
                errors: {
                    editError: "Error editing voter",
                    editSuccess: "Voter edited",
                    createError: "Error creating voter",
                    createSuccess: "Voter created",
                },
                delete: {
                    body: "Are you sure you want to delete this voter?",
                    bulkBody: "Are you sure you want to delete the selected voters?",
                },
                notifications: {
                    deleteError: "Error deleting voter",
                    deleteSuccess: "Voter deleted",
                },
            },
            roles: {
                title: "Roles",
                edit: {
                    title: "Role Data",
                    subtitle: "View and edit role",
                },
                create: {
                    title: "Role",
                    subtitle: "Create role",
                },
                errors: {
                    createError: "Error creating role",
                    createSuccess: "Role created",
                },
                fields: {
                    name: "Name",
                },
                delete: {
                    body: "Are you sure you want to delete this role?",
                },
                notifications: {
                    deleteError: "Error deleting role",
                    deleteSuccess: "Role deleted",
                    permissionEditError: "Error editing permission",
                    permissionEditSuccess: "Permission edited",
                },
            },
            permissions: {
                "tenant-create": "Create Tenant",
                "tenant-read": "Read Tenant",
                "tenant-write": "Edit Tenant",
                "election-event-create": "Create Election Event",
                "election-event-read": "Read Election Event",
                "election-event-write": "Edit Election Event",
                "voter-create": "Create Voter",
                "voter-read": "Read Voter",
                "voter-write": "Edit Voter",
                "user-create": "Create User",
                "user-read": "Read User",
                "user-write": "Edit User",
                "user-permission-create": "Create User Permission",
                "user-permission-read": "Read User Permission",
                "user-permission-write": "Edit User Permission",
                "role-create": "Create Role",
                "role-read": "Read Role",
                "role-write": "Edit Role",
                "role-assign": "Assign Role",
                "communication-template-create": "Create Communication Template",
                "communication-template-read": "Read Communication Template",
                "communication-template-write": "Edit Communication Template",
                "notification-read": "Read Notification",
                "notification-write": "Edit Notification",
                "notification-send": "Send Notification",
                "area-read": "Read Area",
                "area-write": "Edit Area",
                "election-state-write": "Edit Election State",
                "election-type-create": "Create Election Type",
                "election-type-read": "Read Election Type",
                "election-type-write": "Edit Election Type",
                "voting-channel-read": "Read Voting Channel",
                "voting-channel-write": "Edit Voting Channel",
                "trustee-create": "Create Trustee",
                "trustee-read": "Read Trustee",
                "trustee-write": "Edit Trustee",
                "tally-read": "Read Tally",
                "tally-start": "Start Tally",
                "tally-write": "Edit Tally",
                "tally-results-read": "Read Tally Results",
                "publish-read": "Read Publish",
                "publish-write": "Edit Publish",
                "logs-read": "Read Logs",
                "keys-read": "Read Keys",
            },
        },
        common: {
            export: "Export can be a long operation. Are you sure you want to export records?",
            resources: {
                electionEvent: "Election Event",
                election: "Election",
                contest: "Contest",
                candidate: "Candidate",
                noResult: {
                    askCreate: "Do you want to create one?",
                },
            },
            label: {
                add: "Add",
                actions: "Actions",
                create: "Create",
                delete: "Delete",
                archive: "Archive",
                unarchive: "Unarchive",
                cancel: "Cancel",
                edit: "Edit",
                save: "Save",
                close: "Close",
                back: "Back",
                next: "Next",
                warning: "Warning",
                json: "Preview",
                noResult: "No result",
                import: "Import",
                export: "Export",
                loadingData: "Loading data ...",
                exportFormat: "Export '{{item}}' results in {{format}} format",
                allResults: "election event",
                globalAreaResults: "all areas",
            },
            language: {
                es: "Spanish",
                en: "English",
            },
            channel: {
                online: "Online",
                kiosk: "Kiosk",
            },
            message: {
                delete: "Are you sure you want to delete this item?",
            },
        },
        createResource: {
            electionEvent: "Create an Election Event",
            election: "Create an Election",
            contest: "Create a Contest",
            candidate: "Create a Candidate",
        },
        sideMenu: {
            electionEvents: "Election Events",
            search: "Search",
            usersAndRoles: "Users and Roles",
            logs: "Logs",
            settings: "Settings",
            communicationTemplates: "Communication Templates",
            active: "Active",
            archived: "Archived",
            addResource: {
                electionEvent: "Create an Election Event",
                election: "Create an Election",
                contest: "Create a Contest",
                candidate: "Create a Candidate",
            },
            menuActions: {
                archive: {
                    electionEvent: "Archive this Election Event",
                },
                unarchive: {
                    electionEvent: "Unarchive this Election Event",
                    election: "Unarchive this election",
                    contest: "Unarchive this Contest",
                    candidate: "Unarchive this Candidate",
                },
                remove: {
                    electionEvent: "Remove this Election Event",
                    election: "Remove this Election",
                    contest: "Remove this Contest",
                    candidate: "Remove this Candidate",
                },
                messages: {
                    confirm: {
                        archive: "Are you sure to archive this item?",
                        unarchive: "Are you sure to unarchive this item?",
                        delete: "Are you sure to delete this item?",
                    },
                    notification: {
                        success: {
                            archive: "The item has been archived",
                            unarchive: "The item has been unarchived",
                            delete: "The item has been deleted",
                        },
                        error: {
                            archive: "Error while trying to archive this item",
                            unarchive: "Error while trying to unarchive this item",
                            delete: "Error while trying to delete this item",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Candidate configuration.",
            },
            edit: {
                general: "General",
                type: "Type",
                image: "Image",
            },
            field: {
                name: "Name",
                alias: "Alias",
                description: "Description",
            },
            options: {
                "candidate": "Candidate",
                "option": "Option",
                "write-in": "Write-in",
                "open-list": "Open List",
                "closed-list": "Closed List",
                "semi-open-list": "Semi Open List",
                "invalid-vote": "Invalid Vote",
                "blank-vote": "Blank Vote",
            },
            error: {},
            createCandidateSuccess: "Candidate created",
            createCandidateError: "Error creating candidate",
        },
        contestScreen: {
            common: {
                subtitle: "Contest configuration.",
            },
            edit: {
                general: "General",
                type: "Type",
                image: "Image",
                system: "Ballot Voting System",
                design: "Ballot Design",
                reorder: "Reorder candidates",
            },
            field: {
                name: "Name",
                alias: "Alias",
                description: "Description",
            },
            options: {
                "no-preferential": "No Preferential",
                "plurality-at-large": "Plurality at Large",
                "random-asnwers": "Random Answers",
                "custom": "Custom",
                "alphabetical": "Alphabetical",
            },
            error: {},
            createContestSuccess: "Contest created",
            createContestError: "Error creating candidate",
        },
        keysGeneration: {
            configureStep: {
                create: "Create Keys Ceremony",
                title: "Create Election Event Keys Ceremony",
                subtitle:
                    "In the Keys Ceremony each trustee will generate and download their fragment of the private key for the Election Event. To proceed, please choose the trustees that will participate in the ceremony and the threshold, which is the minimum number of trustees required to tally.",
                threshold: "Threshold",
                trusteeList: "Trustees",
                errorMinTrustees:
                    "You selected only {{selected}} trustees, but you must select at least {{threshold}}.",
                errorThreshold:
                    "You selected threshold {{selected}} but it must be between {{min}} and {{max}}.",
                errorCreatingCeremony: "Error creating Keys Ceremony: {{error}}",
                createCeremonySuccess: "Keys Ceremony created",
                confirmdDialog: {
                    ok: "Yes, Create Keys Ceremony",
                    cancel: "Cancel",
                    title: "Are you sure you want to Create Keys Ceremony?",
                    description:
                        "You are about to Create Keys Ceremony. This action will notify the Trustees to participate in the creation and distribution of the Election Event Keys.",
                },
            },
            ceremonyStep: {
                cancel: "Cancel Keys Ceremony",
                progressHeader: "Keys Ceremony Progress",
                description:
                    "This screen shows the progress and logs of the Election Event's Keys Ceremony. In the Keys Ceremony each trustee will generate and download their fragment of the private key for the Election Event.",
                executionStatus: "Status: {{status}}",
                confirmdDialog: {
                    ok: "Yes, Cancel Create Keys Ceremony",
                    cancel: "Go back to Keys Ceremony",
                    title: "Are you sure you want to Cancel Keys Ceremony?",
                    description:
                        "You are about to Cancel Keys Ceremony. After performing this action, to have a succcessful Keys Ceremony you will have to Create a new one.",
                },
                header: {
                    trusteeName: "Trustee Name",
                    fragment: "Key Fragment Generated",
                    downloaded: "Private Key Fragment Downloaded",
                    checked: "Private Key Fragment Checked",
                },
                logsHeader: {
                    title: "Logs",
                    date: "Date",
                    entry: "Entry",
                },
                emptyLogs: "No logs yet.",
            },
            startStep: {
                title: "Trustee Keys Ceremony",
                subtitle:
                    "You are about to participate in the Keys Ceremony as a Trustee (<strong>{{name}}</strong>). This involves the following steps:",
                one: "<strong>Download</strong> your Encrypted Private Key.",
                two: "Create multiple <strong>Backups</strong> of the Encrypted Private Key.",
                three: "<strong>Check</strong> that the backups works well.",
            },
            downloadStep: {
                title: "Download Encrypted Private Key",
                subtitle:
                    "To continue, please download and store your Encrypted Private Key at least into two different devices:",
                downloadButton: "Download your Encrypted Private Key",
                errorDownloading: "Download error: {{error}}",
                errorEmptyKey: "Download error, empty file",
                confirmdDialog: {
                    ok: "Confirm Backups and Continue",
                    cancel: "Go Back",
                    title: "Backup your Encrypted Private Key",
                    description:
                        "Please backup your Encrypted Private Key in at least two different secure locations and then confirm it below:",
                    firstCopy: "First backup secured",
                    secondCopy: "Second backup secured",
                },
            },
            checkStep: {
                title: "Check your Encrypted Private Key Backups",
                subtitle:
                    "Upload a Encrypted Private Key Backup to check that it's correct. You can try as many times as needed, from your different backups:",
                errorUploading: "Upload error: {{error}}",
                errorEmptyFile: "File empty or not found",
                verified: "Backup verified successfully.",
            },
        },
        tally: {
            ceremonyTitle: "Elections to Tally",
            ceremonySubTitle: "Choose the elections you want to tally",
            tallyTitle: "Elections Tally Progress",
            logsTitle: "Logs",
            resultsTitle: "Results & Participation",
            generalInfoTitle: "General Information",
            trusteeTallyTitle: "Trustees",
            trusteeTallySubTitle: "Key fragment import status",
            createTallySuccess: "Tally created",
            createTallyError: "Could not create Tally",
            startTallySuccess: "Tally started",
            startTallyError: "Could not start Tally",
            startTallyCeremonySuccess: "Tally Ceremony started",
            startTallyCeremonyError: "Could not start Tally Ceremony",
            cancelTallyCeremonySuccess: "Tally Ceremony canceled",
            cancelTallyCeremonyError: "Could not cancel Tally Ceremony",
            trusteeTitle: "Trustees process",
            trusteeSubTitle: "Please upload you key fragment",
            invited: "You have been invited to participate in a Tally ceremony. Please ",
            click: "click on the tally Action",
            participate: "to participate.",
            breadcrumbSteps: {
                start: "Start",
                finish: "Finish",
                tally: "Tally",
                results: "Results",
                ceremony: "Ceremony",
            },
            common: {
                title: "Tally",
                subTitle: "Tally configuration.",
                cancel: "Back",
                next: "Next",
                date: "Tally Date",
                global: "Global",
                noTrustees: "No trustees yet",
                imported: " trustees imported the key",
                needed: " trustees needed",
                start: "Start Tally",
                ceremony: "Start Tally Ceremony",
                results: "Results",
                dialog: {
                    ok: "Ok",
                    okTally: "Start Tally",
                    okCancel: "Cancel Tally",
                    cancel: "Close",
                    title: "Are you sure you want to  start a ceremony?",
                    tallyTitle: "Are you sure you want to  start the tally?",
                    cancelTitle: "Are you sure you want to  cancel the tally?",
                    message:
                        "You are about to start a tally ceremony . This action will notify the trustees to import their key fragments.",
                    cancelMessage:
                        "You are about to cancel the tally ceremony. This action is not undoable.",
                    ceremony:
                        "All required trustees have verified their key fragments. Everything is ready to begin receiving results. Do you want to start the Tally?",
                },
            },
            table: {
                elections: "Elections",
                selected: "Selected",
                status: "Status",
                progress: "Progress",
                method: "Tally Method",
                elegible: "Elegible Voters",
                number: "Number of Votes",
                total: "Total",
                turnout: "%",
                candidates: "Candidate Results",
                options: "Options",
                global: "Participation Summary",
                elegible_census: "Elegible Voters",
                number_votes: "Number of Votes",
                total_valid_votes: "Total Valid Votes",
                explicit_invalid_votes: "Explicitly Invalid Votes",
                implicit_invalid_votes: "Implicitly Invalid Votes",
                blank_votes: "Blank Votes",
                number_of_votes: "Number of Votes",
                voters: "Winning position",
            },
        },
        publish: {
            header: {
                change: "Changes to be Publish",
                history: "Publish History",
            },
            action: {
                start: "START ELECTION",
                stop: "STOP ELECTION",
                pause: "PAUSE",
                generate: "REGENERATE",
                publish: "PUBLISH CHANGES",
                back: "BACK",
            },
            label: {
                current: "Current",
                diff: "CHANGES TO PUBLISH",
            },
            empty: {
                header: "No Publication Yet.",
                action: "Generate Publication",
            },
            dialog: {
                title: "Confirm action",
                info: "You have clicked on a sensitive action, so we need you to confirm in order to continue",
                ok: "Confirm",
                ko: "Cancel",
                error: "Error loading ballot publication",
                error_publish: "Error publishing ballot publication",
                error_status: "Error change ballot publication status",
            },
            notifications: {
                generated: "Ballot generated",
                published: "Ballot published",
                chang_status: "Ballot status changed",
            },
        },
        emailEditor: {
            subject: "Email Subject",
            tabs: {
                plaintext: "Plain Text Body",
                richtext: "Rich Text Body",
            },
        },
        sendCommunication: {
            send: "Send",
            title: "Send Notification",
            subtitle: "Send a notification to voters.",
            sendButton: "Send Notification",
            voters: "Audience",
            schedule: "Schedule",
            nowInput: "Send now",
            dateInput: "Date and time to start sending notifications",
            chooseDate: "Please choose a date",
            languages: "Languages",
            smsMessage: "SMS Message",
            errorSending: "Error sending the notification: {{error}}",
            successSending: "Notification programmed/sent successfully",
            votersSelection: {
                ALL_USERS: "Everyone",
                NOT_VOTED: "Those who didn't vote yet",
                VOTED: "Those who already voted",
                SELECTED: "To {{total}} Selected Voters",
            },
            methodTitle: "Communication Method",
            communicationMethod: {
                EMAIL: "Email",
                SMS: "SMS",
            },
            email: {
                subject: "Subject",
            },
        },
        communicationTemplate: {
            empty: {
                title: 'No Communication Template Yet',
                subtitle: 'Do you want to create one?'
            },
            action: {
                createOne: 'Create Communication Template'
            }
        }
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
