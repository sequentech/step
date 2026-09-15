import assert from 'node:assert/strict'
import {chromium} from 'playwright-core'

// Run only against a disposable, seeded full STEP installation. No request mocks.
const required = (name) => {
    assert(process.env[name], `Missing ${name}`)
    return process.env[name]
}
const baseURL = required('PORTAL_URL')
const endpoint = required('HASURA_URL')
const secret = required('HASURA_ADMIN_SECRET')
const eventId = required('EVENT_ID')
const firstId = required('ORIGINAL_FIRST_ID')
const targetId = required('TARGET_FIRST_ID')
const targetName = required('TARGET_FIRST_NAME')
const originalName = required('ORIGINAL_FIRST_NAME')
const broken = process.env.EXPECT_BROKEN === 'true'
assert.equal(process.env.ALLOW_FIXTURE_RESET, 'true', 'Explicit disposable fixture reset required')
async function graphql(query, variables = {}) {
    const res = await fetch(endpoint, {method: 'POST', headers: {'content-type': 'application/json', 'x-hasura-admin-secret': secret}, body: JSON.stringify({query, variables})})
    assert.equal(res.ok, true)
    const result = await res.json()
    assert.equal(result.errors, undefined, JSON.stringify(result.errors))
    return result.data
}
const read = async () => (await graphql(`query($id: uuid!) { sequent_backend_election(where: {election_event_id: {_eq: $id}}) { id presentation } }`, {id:eventId})).sequent_backend_election
const initial = await read()
assert.equal(initial.length, 2, 'Use an isolated two-election fixture')
for (const election of initial) {
    await graphql(`mutation($id: uuid!, $presentation: jsonb!) { update_sequent_backend_election(where: {id: {_eq: $id}}, _set: {presentation: $presentation}) { affected_rows } }`, {id:election.id,presentation:{...election.presentation,sort_order:null}})
}
const browser = await chromium.launch({channel:'chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']})
try {
    const page = await browser.newPage({viewport:{width:1600,height:1000}})
    await page.goto(baseURL, {waitUntil:'domcontentloaded', timeout:120000})
    await page.locator('#username').fill(required('PORTAL_USERNAME'))
    await page.locator('#password').fill(required('PORTAL_PASSWORD'))
    await page.locator('#kc-login').click()
    async function openDesign() {
        await page.getByRole('tab',{name:'Data',exact:true}).click({timeout:60000})
        await page.getByRole('button',{name:'Ballot Design',exact:false}).click()
        await page.locator('[draggable=true]').first().waitFor()
    }
    await openDesign()
    const rows = page.locator('[draggable=true]')
    assert.equal(await rows.count(), 2)
    assert((await rows.first().innerText()).includes(originalName))
    await rows.filter({hasText:targetName}).dragTo(rows.filter({hasText:originalName}))
    assert((await rows.first().innerText()).includes(targetName))
    await page.getByRole('button',{name:'Save',exact:true}).click()
    // Poll the real backend; the broken version never writes election positions.
    let persisted
    for(let i=0;i<20;i++) {
        await page.waitForTimeout(500)
        persisted = await read()
        if(persisted.find(e=>e.id===targetId).presentation.sort_order===0) break
    }
    assert.equal(persisted.find(e=>e.id===targetId).presentation.sort_order,broken ? null : 0)
    assert.equal(persisted.find(e=>e.id===firstId).presentation.sort_order,broken ? null : 1)
    await page.reload()
    await openDesign()
    assert((await rows.first().innerText()).includes(broken ? originalName : targetName))
    console.log(JSON.stringify({expectedBroken:broken,persisted:persisted.map(e=>({id:e.id,sort_order:e.presentation.sort_order})),reloadVerified:true}))
} finally { await browser.close() }
