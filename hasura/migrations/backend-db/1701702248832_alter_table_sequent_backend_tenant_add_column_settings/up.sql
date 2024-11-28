alter table "sequent_backend"."tenant" add column "settings" jsonb
 null;

UPDATE "sequent_backend"."tenant"
SET settings = '{"i18n":{"en":{"stats.areas":"Posts","areas.common.title":"Posts","tally.trusteeTitle":"SBEI","tally.trusteeTallyTitle":"SBEI","electionEventScreen.tabs.areas":"Posts","publish.action.stopVotingPeriod":"Close Election","electionEventScreen.tally.trustees":"SBEI","keysGeneration.configureStep.trusteeList":"SBEI"}},"help_links":[{"url":"https://docs.google.com/document/d/1opTyXioucIgb6UPQ7Teq3gmAVi6U9O5-HgbZp52BZbY/edit?usp=sharing","i18n":{"en":{"title":"System Manual"},"es":{"title":"Manual del Sistema"},"fr":{"title":"System Manual"},"tl":{"title":"System Manual"},"cat":{"title":"System Manual"}},"title":"System Manual"}],"language_conf":{"default_language_code":"en","enabled_language_codes":["en","tl"]},"enroll_countries":[],"voting_countries":["PH"]}',
annotations = '{"css":"header .header-class img {\n    margin-left: 80px;\n    width: 100px !important;\n    height: 100px !important;\n}\n\nmain .MuiDrawer-root.MuiDrawer-docked.RaSidebar-docked .MuiList-root {\n    margin-top: 50px;\n}","logo_url":"https://upload.wikimedia.org/wikipedia/commons/thumb/7/73/Commission_on_Elections_%28COMELEC%29.svg/1280px-Commission_on_Elections_%28COMELEC%29.svg.png"}'
WHERE id = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

