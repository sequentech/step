// delete candidate one
let menu = await browser
    .element(`a[title='this is candidate one name'] + div.menu-actions-${this.candidateLink!}`)
    .moveTo()
browser.click(menu)
browser.element(`li.menu-action-delete-${this.candidateLink!}`).click()
browser.element(`button.ok.button`).click()

// delete candidate two
menu = await browser
    .element(`a[title='this is candidate two name'] + div.menu-actions-${this.candidateLink!}`)
    .moveTo()
browser.click(menu)
browser.element(`li.menu-action-delete-${this.candidateLink!}`).click()
browser.element(`button.ok.button`).click()

// delete contest
menu = await browser
    .element(`a[title='this is contest name'] + div.menu-actions-${this.candidateLink!}`)
    .moveTo()
browser.click(menu)
browser.element(`li.menu-action-delete-${this.contestLink!}`).click()
browser.element(`button.ok.button`).click()

// delete election
menu = await browser
    .element(`a[title='this is election name'] + div.menu-actions-${this.candidateLink!}`)
    .moveTo()
browser.click(menu)
browser.element(`li.menu-action-delete-${this.electionLink!}`).click()
browser.element(`button.ok.button`).click()

// delete election event
menu = await browser
    .element(`a[title='this is celection event name'] + div.menu-actions-${this.candidateLink!}`)
    .moveTo()
browser.click(menu)
browser.element(`li.menu-action-delete-${this.electionEventLink!}`).click()
browser.element(`button.ok.button`).click()
