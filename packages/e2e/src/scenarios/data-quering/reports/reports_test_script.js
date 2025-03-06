// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

client => {
	const url = "https://admin-portal-qa.sequent.vote/sequent_backend_election_event/e3820bf4-26c9-4664-acc5-f352cb2a34fb";
    const admin = "emsov";
    const reportsListTable = "div[class='RaList-main']"
    const actionsMenu = "ul[role='menu']"

    const login = (username, password) => {
        client.waitForElementVisible('body', 20e3)
        .waitForElementVisible('input[name=username]', 10e3)
        .setValue('input[name=username]', username)
        .waitForElementVisible('input[name=password]', 10e3)
        .setValue('input[name=password]', password)
        .click('#kc-login')
    }

    const findButtonReportsActionsAndClick = () => {
        client.execute(function() {
          const buttons = document.querySelectorAll('#actions-menu-button');
          return buttons.length ? buttons[0] : -1;
        },[], function(result) {
        if (result.value === -1) {
          client.assert.fail(`no button with 'Start Tally Ceremony' found`);
        } else {
            result.value.click()
        }
        })
    }

    const findButtonReportsGenerateActionAndClick = () => {
        client.execute(function() {
          const liButtons = document.querySelectorAll("li")
          console.log("find liButtons: ",liButtons);
          for (let i = 0; i < liButtons.length; i++) {
          if (liButtons[i].textContent.includes('Generate')) {
              return liButtons[i]
          }
        }
        return -1
        },[], function(result) {
        if (result.value === -1) {
          client.assert.fail(`no button with 'Generate' found`);
        } else {
            console.log(`Clicked the 'Generate' button`);
            result.value.click()
        }
        })
      }


    const findAndClickReportsTab = () => {
    client.waitForElementVisible("button[role='tab']", 20e3)
    .execute(function() {
        const tabs = document.querySelectorAll("button[role='tab']");
        console.log(tabs)
        for (let i = 0; i < tabs.length; i++) {
        if (tabs[i].textContent.includes('Reports')) {
            tabs[i].click()
            return i;
        }
      }
        return -1
      },[], function(result) {
      if (result.value === -1) {
        client.assert.fail("No tab containing text 'Reports' was found!");
      } else {
        console.log(`Clicked the 'Reports' tab at index: ${result.value}`);
      }
      })
    }

    client
        .url(url)
        login(admin, admin)
        client
        .pause(1000)
        .saveScreenshot("screenshots/home_page.png")
        findAndClickReportsTab()
        client.pause(500)
        .waitForElementVisible(reportsListTable, 10e3)
        .saveScreenshot(`screenshots/report_tab.png`)
        findButtonReportsActionsAndClick()
        client.pause(500)
        .waitForElementVisible(actionsMenu, 10e3)
        .saveScreenshot("screenshots/reports_actions_menu.png")
        findButtonReportsGenerateActionAndClick()
}