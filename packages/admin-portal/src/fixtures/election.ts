// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IElectionDTO} from "sequent-core"

export const ELECTION_CATEGORIES: IElectionDTO = {
    id: 34570004,
    configuration: {
        id: 34570004,
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "With categories",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
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
                        id: 0,
                        category: "A",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [],
                        text: "Example option 1",
                    },
                    {
                        id: 1,
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
                        id: 2,
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
                        id: 3,
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
                        id: 4,
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
    pks: '[{"q":"24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723","p":"49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447","y":"7935686013232753543311730853542516395387132248199082005324975648969547902701098719629058754324589496219336121014697696843786095500202513391568290500700390170757903766134406266478846034766577758369116832449690998471704057640699323780294813255022425208363156900818184156862551888221865453786194965318602957154571823164459984267811924669480617298217085181904368825833339725551767399965948648204274314963026937232756975690252280458979907624812073239149681479201670717850347818870249000411502313070025806821586930845325247143507359739295343115889688545659443947034348548413050217024033012253872883631238224552306584054707","g":"27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147"}]',
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
    id: 34570002,
    configuration: {
        id: 34570002,
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "Simple election plurality",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
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
                        id: 0,
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
                        id: 1,
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
                        id: 2,
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
    pks: '[{"q":"24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723","p":"49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447","y":"3192515660619108169365014720875495689510715604656883748612343903823995403425790631339251205893892434659355505746651700479054838604440185058106195965495195908647156080974534267530239429053856517521625301507367856050050239682969661184674721546490963923812051964716874579616239470644906006000375420650293188431637635599268098297680517168870031026335774002504842468275787458511150985008555696193983445399118752579412134160463549182761208081376034003185079445259071999506915005558311361669768641321142183454952311775988265966400753950365761816295744457795066374773015741196128191865863027069409284986816791016922657728787","g":"27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147"}]',
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
    id: 34570005,
    configuration: {
        id: 34570005,
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "Write-ins simple",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
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
                        id: 0,
                        category: "",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [],
                        text: "Example option 1",
                    },
                    {
                        id: 1,
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
                        id: 2,
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
                        id: 3,
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
    pks: '[{"q":"24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723","p":"49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447","y":"39138053529638475255028067842416470700950283630297285560925136019696810245867147891653418869111330214848118557163300732640643485353179833867483833289937820879952123617961224785095533514881103501185645387300222139882444882214376695125516295351498010367837729500700028951801381485130233396302138102166566597973601533867774150847073948239463357713618034258079858550684778452193130853213983134215776686524972254275783087651251809357926681935837893498820622976148799104989568022857201105862005024590401928876184602077371257335033309874504908920624709536648172095439489053401247526472138953860322758925786690992804516542097","g":"27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147"}]',
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
    id: 34570006,
    configuration: {
        id: 34570006,
        layout: "simple",
        director: "6xx-a1",
        authorities: ["6xx-a2"],
        title: "With Invalid Vote",
        description:
            'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.',
        questions: [
            {
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
                        id: 0,
                        category: "",
                        details: "This is an option with an simple example description.",
                        sort_order: 0,
                        urls: [],
                        text: "Example option 1",
                    },
                    {
                        id: 1,
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
                        id: 2,
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
                        id: 3,
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
    pks: '[{"q":"24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723","p":"49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447","y":"39138053529638475255028067842416470700950283630297285560925136019696810245867147891653418869111330214848118557163300732640643485353179833867483833289937820879952123617961224785095533514881103501185645387300222139882444882214376695125516295351498010367837729500700028951801381485130233396302138102166566597973601533867774150847073948239463357713618034258079858550684778452193130853213983134215776686524972254275783087651251809357926681935837893498820622976148799104989568022857201105862005024590401928876184602077371257335033309874504908920624709536648172095439489053401247526472138953860322758925786690992804516542097","g":"27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147"}]',
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
