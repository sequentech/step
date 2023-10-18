use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

pub struct TestFixture {
    pub config_path: PathBuf,
    pub root_dir: String,
    pub input_dir_configs: String,
    pub input_dir_ballots: String,
}

impl TestFixture {
    pub fn new() -> Result<Self> {
        let config_path = PathBuf::from(format!("test-velvet-config-{}.json", Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&config_path)?;

        writeln!(file, "{}", get_config())?;

        let root_dir = format!("./tests-input__{}", Uuid::new_v4());
        let input_dir_configs = format!("{}/tests/input-dir/default/configs", &root_dir);
        let input_dir_ballots = format!("{}/tests/input-dir/default/ballots", &root_dir);

        fs::create_dir_all(&input_dir_configs)?;
        fs::create_dir_all(&input_dir_ballots)?;

        Ok(Self {
            config_path,
            root_dir,
            input_dir_configs,
            input_dir_ballots,
        })
    }

    pub fn create_election_config(&self) -> Result<Uuid> {
        let uuid = Uuid::new_v4();

        let dir = format!("{}/election__{}", self.input_dir_configs, uuid);
        fs::create_dir_all(&dir)?;
        let mut file = fs::File::create(format!("{}/election-config.json", dir))?;

        let dir = format!("{}/election__{}", self.input_dir_ballots, uuid);
        fs::create_dir_all(dir)?;

        writeln!(file, "{}", get_election_config())?;

        Ok(uuid)
    }

    pub fn create_contest_config(&self, election_uuid: &Uuid) -> Result<Uuid> {
        let uuid = Uuid::new_v4();

        let dir = format!(
            "{}/election__{}/contest__{}",
            self.input_dir_configs, election_uuid, uuid
        );

        fs::create_dir_all(&dir)?;
        let mut file = fs::File::create(format!("{}/contest-config.json", dir))?;

        let dir = format!(
            "{}/election__{}/contest__{}",
            self.input_dir_ballots, election_uuid, uuid
        );
        fs::create_dir_all(dir)?;

        writeln!(file, "{}", get_contest_config())?;

        Ok(uuid)
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        fs::remove_file(&self.config_path).unwrap();
        fs::remove_dir_all(&self.root_dir).unwrap();
    }
}

fn get_config() -> String {
    let config_content = r#"
        {
            "version": "0.0.0",
            "stages": {
                "order": ["main"],
                "main": {
                    "pipeline": [
                        {
                            "id": "decode-ballots",
                            "pipe": "VelvetDecodeBallots",
                            "config": {}
                        },
                        {
                            "id": "do-tally",
                            "pipe": "VelvetDoTally",
                            "config": {
                                "invalidateVotes": "Fail"
                            }
                        },
                        {
                            "id": "consolidation",
                            "pipe": "VelvetConsolidation",
                            "config": {}
                        },
                        {
                            "id": "ties-resolution",
                            "pipe": "VelvetTiesResolution",
                            "config": {}
                        },
                        {
                            "id": "compute-result",
                            "pipe": "VelvetComputeResult",
                            "config": {}
                        },
                        {
                            "id": "gen-report",
                            "pipe": "VelvetGenerateReport",
                            "config": {
                                "formats": ["pdf", "csv"]
                            }
                        }
                    ]
                }
            }
        }
    "#;

    config_content.to_string()
}

fn get_election_config() -> String {
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

    election_str.to_string()
}

fn get_contest_config() -> String {
    let contest_str = r#"
        {
            "id":"1fc963b1-f93b-4151-93d6-bbe0ea5eac46",
            "description":"Elige quien quieres que sea tu Secretario General en tu municipio",
            "layout":"",
            "min":1,
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

    contest_str.to_string()
}
