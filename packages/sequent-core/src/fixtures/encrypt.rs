// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::BallotStyle;
use crate::plaintext::DecodedVoteContest;

pub fn get_encrypt_decoded_test_fixture(
) -> (Vec<DecodedVoteContest>, BallotStyle) {
    let election_str = r#"{
        "id":34570002,
        "configuration":{
           "id":34570002,
           "layout":"simple",
           "director":"6xx-a1",
           "authorities":[
              "6xx-a2"
           ],
           "title":"New election",
           "description":"This is the description of the election. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.",
           "contests":[
              {
                 "description":"This is the description of this contest. You can have multiple contests. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.",
                 "layout":"simultaneous-contests",
                 "max":3,
                 "min":1,
                 "num_winners":1,
                 "title":"Test contest title",
                 "tally_type":"plurality-at-large",
                 "candidate_total_votes_percentage":"over-total-valid-votes",
                 "candidates":[
                    {
                       "id":0,
                       "category":"",
                       "details":"This is an option with an simple example description.",
                       "sort_order":0,
                       "urls":[
                        {
                           "title":"Image URL",
                           "url":"/XFQwVFL.jpg"
                        }
                       ],
                       "text":"Example option 1"
                    },
                    {
                       "id":1,
                       "category":"",
                       "details":"An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.",
                       "sort_order":1,
                       "urls":[
                          {
                             "title":"URL",
                             "url":"https://sequentech.io"
                          },
                          {
                             "title":"Image URL",
                             "url":"/XFQwVFL.jpg"
                          }
                       ],
                       "text":"Example option 2"
                    },
                    {
                       "id":2,
                       "category":"",
                       "details":"",
                       "sort_order":2,
                       "urls":[
                          
                       ],
                       "text":"Example option 3"
                    }
                 ],
                 "extra_options":{
                    "shuffle_categories":true,
                    "shuffle_category_list":[
                       
                    ],
                    "show_points":false
                 }
              }
           ],
           "presentation":{
              "share_text":[
                 {
                    "network":"Twitter",
                    "button_text":"",
                    "social_message":"I have just voted in election __URL__, you can too! #sequent"
                 }
              ],
              "theme":"default",
              "urls":[
                 
              ],
              "theme_css":""
           },
           "extra_data":"{}",
           "tallyPipesConfig":"{\"version\":\"master\",\"pipes\":[{\"type\":\"tally_pipes.pipes.results.do_tallies\",\"params\":{}},{\"type\":\"tally_pipes.pipes.sort.sort_non_iterative\",\"params\":{}}]}",
           "ballotBoxesResultsConfig":"",
           "virtual":false,
           "tally_allowed":false,
           "publicCandidates":true,
           "virtualSubelections":[
              
           ],
           "logo_url":""
        },
        "state":"created",
        "pks":"[{\"q\":\"24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723\",\"p\":\"49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447\",\"y\":\"3192515660619108169365014720875495689510715604656883748612343903823995403425790631339251205893892434659355505746651700479054838604440185058106195965495195908647156080974534267530239429053856517521625301507367856050050239682969661184674721546490963923812051964716874579616239470644906006000375420650293188431637635599268098297680517168870031026335774002504842468275787458511150985008555696193983445399118752579412134160463549182761208081376034003185079445259071999506915005558311361669768641321142183454952311775988265966400753950365761816295744457795066374773015741196128191865863027069409284986816791016922657728787\",\"g\":\"27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147\"}]",
        "tallyPipesConfig":"{\"version\":\"master\",\"pipes\":[{\"type\":\"tally_pipes.pipes.results.do_tallies\",\"params\":{}},{\"type\":\"tally_pipes.pipes.sort.sort_non_iterative\",\"params\":{}}]}",
        "ballotBoxesResultsConfig":"",
        "virtual":false,
        "tallyAllowed":false,
        "publicCandidates":true,
        "logo_url":"",
        "trusteeKeysState":[
           {
              "id":"6xx-a1",
              "state":"initial"
           },
           {
              "id":"6xx-a2",
              "state":"initial"
           }
        ]
    }"#;
    let decoded_contests_str = r#"[
        {
            "is_explicit_invalid": false,
            "invalid_errors": [],
            "choices": [
                {
                    "id": 0,
                    "selected": 0
                },
                {
                    "id": 1,
                    "selected": -1
                },
                {
                    "id": 2,
                    "selected": 0
                }
            ]
        }
    ]"#;
    let election: BallotStyle = serde_json::from_str(election_str).unwrap();
    let decoded_contests: Vec<DecodedVoteContest> =
        serde_json::from_str(decoded_contests_str).unwrap();

    (decoded_contests, election)
}

pub fn default_voting_portal_fixture() -> (Vec<DecodedVoteContest>, BallotStyle)
{
    let ballot_selection_str = r#"[{"contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6","is_explicit_invalid":false,"invalid_errors":[],"choices":[{"id":"a24303de-5798-47cd-9b3e-4f391d1bae7b","selected":0},{"id":"d9249345-11be-4652-ad04-298d70931610","selected":-1},{"id":"1822089d-ae17-4a03-8935-25164b3f2142","selected":-1}]}]"#;

    let election_str = r#"{
      "id":"a12b9343-466e-429f-8ab4-99f6e32bf265",
      "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
      "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
      "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
      "description":"This is the description of the election. You can add simple html like You need to use two br element for new paragraphs.",
      "public_key":{
         "public_key":"/jXUkdSIgz8mXLZ4BIDPQzDx7ZFFIG3MWuacDLyhyhoCAAAAGORKDU/t+8fKNkZMFfXl1IMM+/0VmINTZCcbalZ/NSUi5SbzUTlyzh25lMuVALwvC/lk3j6SHn6BotYphk0QMA",
         "is_demo":true
      },
      "area_id":"2f312a36-f39c-46e4-9670-1d1ce4625745",
      "status":null,
      "contests":[
         {
            "id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
            "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
            "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
            "name":"Who's the best president of the USA?",
            "description":"Choose a president",
            "max_votes":1,
            "min_votes":1,
            "voting_type":"first-past-the-post",
            "counting_algorithm":"plurality-at-large",
            "is_encrypted":true,
            "candidates":[
               {
                  "id":"a24303de-5798-47cd-9b3e-4f391d1bae7b",
                  "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                  "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
                  "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                  "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
                  "name":"Joe Biden",
                  "description":"The current president",
                  "candidate_type":null,
                  "presentation":null
               },
               {
                  "id":"d9249345-11be-4652-ad04-298d70931610",
                  "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                  "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
                  "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                  "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
                  "name":"Donald Trump",
                  "description":"A right-wing populist",
                  "candidate_type":null,
                  "presentation":null
               },
               {
                  "id":"1822089d-ae17-4a03-8935-25164b3f2142",
                  "tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                  "election_event_id":"33f18502-a67c-4853-8333-a58630663559",
                  "election_id":"f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
                  "contest_id":"69f2f987-460c-48ac-ac7a-4d44d99b37e6",
                  "name":"Barrak Obama",
                  "description":"First Black president and very charismatic",
                  "candidate_type":null,
                  "presentation":null
               }
            ],
            "presentation":null
         }
      ]
   }"#;

    let decoded_contests: Vec<DecodedVoteContest> =
        serde_json::from_str(ballot_selection_str).unwrap();
    let election: BallotStyle = serde_json::from_str(election_str).unwrap();

    (decoded_contests, election)
}
