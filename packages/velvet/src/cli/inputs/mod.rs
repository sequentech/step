#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipes::decode_ballots::BallotCodec;
    use anyhow::Result;
    use std::fs::{self, File};
    use std::io::Write;
    use uuid::Uuid;

    struct TestFixture {
        root_dir: String,
        input_dir_configs: String,
        input_dir_ballots: String,
    }

    /*
    ./path/to/input-dir/default/configs/
    |-- election__<uuid>/
        |-- election-config.json
        |-- contest__<uuid>/
            |-- contest-config.json
            |-- area__<uuid>/
                |-- area-config.json

    ./path/to/input-dir/default/ballots/
    |-- election__<uuid>/
        |-- contest__<uuid>/
            |-- area__<uuid>/
                |-- ballots.csv
    */

    impl Drop for TestFixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root_dir).unwrap();
        }
    }

    impl TestFixture {
        fn new() -> Result<Self> {
            let root_dir = "./tests-input".to_string();
            let input_dir_configs = format!("{}/tests/input-dir/default/configs", &root_dir);
            let input_dir_ballots = format!("{}/tests/input-dir/default/ballots", &root_dir);

            fs::create_dir_all(&input_dir_configs)?;
            fs::create_dir_all(&input_dir_ballots)?;

            Ok(Self {
                root_dir,
                input_dir_configs,
                input_dir_ballots,
            })
        }

        fn create_election_config(&self) -> Result<Uuid> {
            let uuid = Uuid::new_v4();

            let dir = format!("{}/election__{}", self.input_dir_configs, uuid);
            fs::create_dir_all(&dir)?;
            let mut file = File::create(format!("{}/election-config.json", dir))?;

            let election_str = r#"
            {
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
                "questions":[
                    {
                        "description":"This is the description of this question. You can have multiple questions. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.",
                        "layout":"simultaneous-questions",
                        "max":3,
                        "min":1,
                        "num_winners":1,
                        "title":"Test question title",
                        "tally_type":"plurality-at-large",
                        "answer_total_votes_percentage":"over-total-valid-votes",
                        "answers":[
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
                            "shuffle_all_options":true,
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
            }
            "#;

            writeln!(file, "{election_str}")?;

            Ok(uuid)
        }

        fn create_contest_config(&self, election_uuid: &Uuid) -> Result<Uuid> {
            let uuid = Uuid::new_v4();

            let dir = format!(
                "{}/election__{}/contest__{}",
                self.input_dir_configs, election_uuid, uuid
            );
            fs::create_dir_all(&dir)?;
            let mut file = File::create(format!("{}/contest-config.json", dir))?;

            let contest_str = r#"
            {
                "id":"1fc963b1-f93b-4151-93d6-bbe0ea5eac46",
                "description":"Elige quien quieres que sea tu Secretario General en tu municipio",
                "layout":"",
                "min":0,
                "max":1,
                "num_winners":1,
                "title":"Secretario General",
                "tally_type":"plurality-at-large",
                "answer_total_votes_percentage":"over-total-valid-votes",
                "answers":[
                    {
                        "id":"0",
                        "category":"Candidaturas no agrupadas",
                        "details":"",
                        "sort_order":0,
                        "urls":[
                            
                        ],
                        "text":"José Rabano Pimiento"
                    },
                    {
                        "id":"1",
                        "category":"Candidaturas no agrupadas",
                        "details":"",
                        "sort_order":1,
                        "urls":[
                            
                        ],
                        "text":"Miguel Pimentel Inventado"
                    },
                    {
                        "category":"Candidaturas no agrupadas",
                        "text":"Juan Iglesias Torquemada",
                        "sort_order":2,
                        "details":"",
                        "urls":[
                            
                        ],
                        "id":"2"
                    },
                    {
                        "category":"Candidaturas no agrupadas",
                        "text":"Mari Pili Hernández Ordoñez",
                        "sort_order":3,
                        "details":"",
                        "urls":[
                            
                        ],
                        "id":"3"
                    },
                    {
                        "category":"Candidaturas no agrupadas",
                        "text":"Juan Y Medio",
                        "sort_order":4,
                        "details":"",
                        "urls":[
                            
                        ],
                        "id":"4"
                    }
                ],
                "extra_options":{
                    "base32_writeins":true
                }
            }
            "#;

            writeln!(file, "{contest_str}")?;

            Ok(uuid)
        }
    }

    #[test]
    fn test_create_election_configs() -> Result<()> {
        let fixture = TestFixture::new()?;

        let uuid = fixture.create_election_config()?;
        fixture.create_contest_config(&uuid)?;
        let uuid = fixture.create_election_config()?;
        fixture.create_contest_config(&uuid)?;
        fixture.create_contest_config(&uuid)?;
        fixture.create_contest_config(&uuid)?;
        fixture.create_contest_config(&uuid)?;
        let uuid = fixture.create_election_config()?;
        fixture.create_contest_config(&uuid)?;
        let uuid = fixture.create_election_config()?;
        fixture.create_contest_config(&uuid)?;
        let uuid = fixture.create_election_config()?;
        fixture.create_contest_config(&uuid)?;

        let entries = fs::read_dir(&fixture.input_dir_configs)?;
        let count = entries.count();

        assert_eq!(count, 5);

        Ok(())
    }

    #[test]
    fn test_ballot_codec() {
        let choices = vec![0, 0, 0, 1, 0, 0];
        let ballot_codec = BallotCodec::new(vec![2, 2, 2, 2, 2, 2]);
        let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
        let decoded_ballot = ballot_codec.decode_ballot(encoded_ballot);

        assert_eq!(decoded_ballot, choices);
    }
}
