// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IElectionDTO} from "sequent-core"

export const ELECTION_CATEGORIES: IElectionDTO = {
    id: "67baa473-e29d-49f0-91b5-da4ea6fa3b3f",
    configuration: {
        id: "67baa473-e29d-49f0-91b5-da4ea6fa3b3f",
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "With categories",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
                id: "0879c2ff-1212-4c9d-a415-907c47a9e28f",
                description:
                    'This is the description of this question. You can have multiple questions. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
                layout: "simultaneous-questions",
                max: 1,
                min: 1,
                num_winners: 1,
                title: "Test question title",
                tally_type: "plurality-at-large",
                answer_total_votes_percentage: "over-total-valid-votes",
                answers: [
                    {
                        id: "780b4eb5-4f83-47b7-a8e0-79343bfa5e43",
                        category: "A",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [],
                        text: "Example option 1",
                    },
                    {
                        id: "e7b58e4d-7297-4ceb-8f87-f93042f91576",
                        category: "A",
                        details:
                            'An option can contain a description. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.',
                        sort_order: 1,
                        urls: [
                            {
                                title: "URL",
                                url: "https://sequentech.io",
                            },
                            {
                                title: "Image URL",
                                url: "/XFQwVFL.jpg",
                            },
                        ],
                        text: "Example option 2",
                    },
                    {
                        id: "d88d4872-49c3-4f7d-9e24-15a0c82efd01",
                        category: "B",
                        details: "",
                        sort_order: 2,
                        urls: [
                            {
                                title: "Image URL",
                                url: "/XFQwVFL.jpg",
                            },
                        ],
                        text: "Example option 3",
                    },
                    {
                        id: "d6d097c5-1079-4050-ac96-8f2fe5f0bb37",
                        category: "A",
                        details: "A Category",
                        sort_order: 3,
                        urls: [
                            {
                                title: "isCategoryList",
                                url: "true",
                            },
                        ],
                        text: "A",
                    },
                    {
                        id: "8b7d848a-8d66-47af-8dbd-a811f5ff9582",
                        category: "B",
                        details: "B Category",
                        sort_order: 4,
                        urls: [
                            {
                                title: "isCategoryList",
                                url: "true",
                            },
                        ],
                        text: "B",
                    },
                ],
                extra_options: {
                    shuffle_categories: true,
                    shuffle_all_options: true,
                    shuffle_category_list: [],
                    show_points: false,
                    enable_checkable_lists: "allow-selecting-candidates-and-lists",
                },
            },
        ],
        presentation: {
            share_text: [
                {
                    network: "Twitter",
                    button_text: "",
                    social_message: "I have just voted in election __URL__, you can too! #sequent",
                },
            ],
            theme: "default",
            urls: [],
            theme_css: "",
        },
        extra_data: "{}",
        tallyPipesConfig:
            '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
        ballotBoxesResultsConfig: "",
        virtual: false,
        tally_allowed: false,
        publicCandidates: true,
        virtualSubelections: [],
        logo_url: "",
    },
    state: "started",
    startDate: "2023-08-06T13:22:14.548",
    public_key: {
        public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
        is_demo: true,
    },
    tallyPipesConfig:
        '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
    ballotBoxesResultsConfig: "",
    virtual: false,
    tallyAllowed: false,
    publicCandidates: true,
    logo_url: "",
    trusteeKeysState: [
        {
            id: "6xx-a1",
            state: "initial",
        },
        {
            id: "6xx-a2",
            state: "initial",
        },
    ],
}

export const SIMPLE_ELECTION_PLURALITY: IElectionDTO = {
    id: "60e4ddce-2d5b-4dea-bbae-e81010aa1f0f",
    configuration: {
        id: "60e4ddce-2d5b-4dea-bbae-e81010aa1f0f",
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "Simple election plurality",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46",
                description:
                    'This is the description of this question. You can have multiple questions. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
                layout: "simultaneous-questions",
                max: 3,
                min: 1,
                num_winners: 1,
                title: "Test question title",
                tally_type: "plurality-at-large",
                answer_total_votes_percentage: "over-total-valid-votes",
                answers: [
                    {
                        id: "38df9caf-2dc8-472c-87f2-f003241e9510",
                        category: "",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [
                            {
                                title: "Image URL",
                                url: "https://i.imgur.com/XFQwVFL.jpg",
                            },
                        ],
                        text: "Example option 1",
                    },
                    {
                        id: "97ac7d0a-e0f5-4e51-a1ee-6614c0836fec",
                        category: "",
                        details:
                            'An option can contain a description. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.',
                        sort_order: 1,
                        urls: [
                            {
                                title: "URL",
                                url: "https://sequentech.io",
                            },
                            {
                                title: "Image URL",
                                url: "/XFQwVFL.jpg",
                            },
                        ],
                        text: "Example option 2",
                    },
                    {
                        id: "94c9eafa-ebc6-4594-a176-24788f761ced",
                        category: "",
                        details: "",
                        sort_order: 2,
                        urls: [],
                        text: "Example option 3",
                    },
                ],
                extra_options: {
                    shuffle_categories: true,
                    shuffle_all_options: true,
                    shuffle_category_list: [],
                    show_points: false,
                },
            },
        ],
        presentation: {
            share_text: [
                {
                    network: "Twitter",
                    button_text: "",
                    social_message: "I have just voted in election __URL__, you can too! #sequent",
                },
            ],
            theme: "default",
            urls: [],
            theme_css: "",
        },
        extra_data: "{}",
        tallyPipesConfig:
            '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
        ballotBoxesResultsConfig: "",
        virtual: false,
        tally_allowed: false,
        publicCandidates: true,
        virtualSubelections: [],
        logo_url: "",
    },
    state: "created",
    public_key: {
        public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
        is_demo: false,
    },
    tallyPipesConfig:
        '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
    ballotBoxesResultsConfig: "",
    virtual: false,
    tallyAllowed: false,
    publicCandidates: true,
    logo_url: "",
    trusteeKeysState: [
        {
            id: "6xx-a1",
            state: "initial",
        },
        {
            id: "6xx-a2",
            state: "initial",
        },
    ],
}

export const ELECTION_WRITEINS_SIMPLE: IElectionDTO = {
    id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8",
    configuration: {
        id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8",
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "Write-ins simple",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
                id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a",
                description:
                    'This is the description of this question. You can have multiple questions. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
                layout: "simultaneous-questions",
                max: 2,
                min: 1,
                num_winners: 1,
                title: "Test question title",
                tally_type: "plurality-at-large",
                answer_total_votes_percentage: "over-total-valid-votes",
                answers: [
                    {
                        id: "f257cd3a-d1cf-4b97-91f8-2dfe156b015c",
                        category: "",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [],
                        text: "Example option 1",
                    },
                    {
                        id: "17325099-f5ab-4c48-a142-6d7ed721e9bb",
                        category: "",
                        details:
                            'An option can contain a description. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.',
                        sort_order: 1,
                        urls: [
                            {
                                title: "URL",
                                url: "https://sequentech.io",
                            },
                            {
                                title: "Image URL",
                                url: "/XFQwVFL.jpg",
                            },
                        ],
                        text: "Example option 2",
                    },
                    {
                        id: "61320aac-0d78-4001-845e-a2f2bd8e800b",
                        category: "",
                        details: "",
                        sort_order: 2,
                        urls: [
                            {
                                title: "isWriteIn",
                                url: "true",
                            },
                        ],
                        text: "",
                    },
                    {
                        id: "e9ad3ed1-4fd5-4498-a0e7-3a3c22ef57d5",
                        category: "",
                        details: "",
                        sort_order: 3,
                        urls: [
                            {
                                title: "isWriteIn",
                                url: "true",
                            },
                        ],
                        text: "",
                    },
                ],
                extra_options: {
                    shuffle_categories: true,
                    shuffle_all_options: true,
                    shuffle_category_list: [],
                    show_points: false,
                    allow_writeins: true,
                },
            },
        ],
        presentation: {
            share_text: [
                {
                    network: "Twitter",
                    button_text: "",
                    social_message: "I have just voted in election __URL__, you can too! #sequent",
                },
            ],
            theme: "default",
            urls: [],
            theme_css: "",
        },
        extra_data: "{}",
        tallyPipesConfig:
            '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
        ballotBoxesResultsConfig: "",
        virtual: false,
        tally_allowed: false,
        publicCandidates: true,
        virtualSubelections: [],
        logo_url: "",
    },
    state: "created",
    public_key: {
        public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
        is_demo: false,
    },
    tallyPipesConfig:
        '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
    ballotBoxesResultsConfig: "",
    virtual: false,
    tallyAllowed: false,
    publicCandidates: true,
    logo_url: "",
    trusteeKeysState: [
        {
            id: "6xx-a1",
            state: "initial",
        },
        {
            id: "6xx-a2",
            state: "initial",
        },
    ],
}

export const ELECTION_WITH_INVALID: IElectionDTO = {
    id: "45b95ddb-e9b7-4a83-ac14-d6fe21574637",
    configuration: {
        id: "45b95ddb-e9b7-4a83-ac14-d6fe21574637",
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "With Invalid Vote",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
                id: "87c19855-00de-4093-b155-6fdfa8a24d42",
                description:
                    'This is the description of this question. You can have multiple questions. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
                layout: "simultaneous-questions",
                max: 2,
                min: 1,
                num_winners: 1,
                title: "Test question title",
                tally_type: "plurality-at-large",
                answer_total_votes_percentage: "over-total-valid-votes",
                answers: [
                    {
                        id: "68fc13bd-22a7-4d24-ab59-f43d9c260cf5",
                        category: "",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [],
                        text: "Example option 1",
                    },
                    {
                        id: "08a4b4a5-5933-4843-ba78-6daf2f655e7c",
                        category: "",
                        details:
                            'An option can contain a description. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.',
                        sort_order: 1,
                        urls: [
                            {
                                title: "URL",
                                url: "https://sequentech.io",
                            },
                            {
                                title: "Image URL",
                                url: "/XFQwVFL.jpg",
                            },
                        ],
                        text: "Example option 2",
                    },
                    {
                        id: "d94e8fc4-94c0-4ddc-ab7c-0302c739d7ad",
                        category: "",
                        details: "",
                        sort_order: 2,
                        urls: [
                            {
                                title: "isWriteIn",
                                url: "true",
                            },
                        ],
                        text: "",
                    },
                    {
                        category: "",
                        details: "",
                        id: "244327fe-8de2-4701-9f64-f114238ff9ce",
                        sort_order: 3,
                        text: "Invalid vote",
                        urls: [
                            {
                                title: "invalidVoteFlag",
                                url: "true",
                            },
                            {
                                title: "positionFlag",
                                url: "top",
                            },
                        ],
                    },
                ],
                extra_options: {
                    shuffle_categories: true,
                    shuffle_all_options: true,
                    shuffle_category_list: [],
                    show_points: false,
                    allow_writeins: true,
                },
            },
        ],
        presentation: {
            share_text: [
                {
                    network: "Twitter",
                    button_text: "",
                    social_message: "I have just voted in election __URL__, you can too! #sequent",
                },
            ],
            theme: "default",
            urls: [],
            theme_css: "",
        },
        extra_data: "{}",
        tallyPipesConfig:
            '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
        ballotBoxesResultsConfig: "",
        virtual: false,
        tally_allowed: false,
        publicCandidates: true,
        virtualSubelections: [],
        logo_url: "",
    },
    state: "created",
    public_key: {
        public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
        is_demo: false,
    },
    tallyPipesConfig:
        '{"version":"master","pipes":[{"type":"tally_pipes.pipes.results.do_tallies","params":{}},{"type":"tally_pipes.pipes.sort.sort_non_iterative","params":{}}]}',
    ballotBoxesResultsConfig: "",
    virtual: false,
    tallyAllowed: false,
    publicCandidates: true,
    logo_url: "",
    trusteeKeysState: [
        {
            id: "6xx-a1",
            state: "initial",
        },
        {
            id: "6xx-a2",
            state: "initial",
        },
    ],
}

export const ELECTIONS_LIST = [
    ELECTION_CATEGORIES,
    SIMPLE_ELECTION_PLURALITY,
    ELECTION_WRITEINS_SIMPLE,
    ELECTION_WITH_INVALID,
]
