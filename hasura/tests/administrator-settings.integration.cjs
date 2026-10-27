// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Explicit opt-in: replaces metadata and creates a fixture in a fresh local
// Hasura/PostgreSQL only. Requests under test use signed bearer JWTs, never the
// administrator secret. The fixture retains the exact selected production role
// permissions; unrelated relationships/tables are not claimed as covered.
const assert = require('node:assert/strict')
const {readFileSync} = require('node:fs')
const {resolve} = require('node:path')
const {createHmac} = require('node:crypto')
const http = require('node:http')
const yaml = require('js-yaml')
const origin = process.env.SETTINGS_TEST_URL
const adminSecret = process.env.SETTINGS_TEST_ADMIN_SECRET
const jwtKey = process.env.SETTINGS_TEST_JWT_KEY
assert.ok(origin && new URL(origin).hostname === '127.0.0.1' && adminSecret && jwtKey,
    'Use SETTINGS_TEST_URL, SETTINGS_TEST_ADMIN_SECRET and SETTINGS_TEST_JWT_KEY for a disposable local server')
const tenant = '11111111-1111-4111-8111-111111111111'
const otherTenant = '22222222-2222-4222-8222-222222222222'
const event = '33333333-3333-4333-8333-333333333333'
const read = path => readFileSync(resolve(__dirname, path), 'utf8')
async function raw(path, body, headers) {
    const res = await fetch(origin + path, {method:'POST',headers:{'content-type':'application/json',...headers},body:JSON.stringify(body),signal:AbortSignal.timeout(5000)})
    return res.json()
}
async function privileged(path, body) {
    const result = await raw(path, body, {'x-hasura-admin-secret':adminSecret})
    assert.ok(!result.error, JSON.stringify(result)); return result
}
const sql = text => privileged('/v2/query',{type:'run_sql',args:{source:'default',sql:text}})
function token(roles, tenantId=tenant) {
    const encode = obj => Buffer.from(JSON.stringify(obj)).toString('base64url')
    const content = encode({alg:'HS256',typ:'JWT'})+'.'+encode({sub:'fixture-human',iat:Math.floor(Date.now()/1000),exp:Math.floor(Date.now()/1000)+300,
      'https://hasura.io/jwt/claims':{'x-hasura-default-role':roles[0],'x-hasura-allowed-roles':roles,'x-hasura-user-id':'44444444-4444-4444-8444-444444444444','x-hasura-tenant-id':tenantId}})
    return content+'.'+createHmac('sha256',jwtKey).update(content).digest('base64url')
}
const admin = token(['admin-user'])
const board = token(['election-event-read'])
async function query(query, bearer=admin, role='admin-user') {
    return raw('/v1/graphql',{query},{authorization:'Bearer '+bearer,'x-hasura-role':role})
}
const denied = result => assert.ok(result.errors?.length || result.error, 'Protected operation unexpectedly succeeded')
async function main() {
    await privileged('/v1/metadata',{type:'replace_metadata',args:{version:3,sources:[{name:'default',kind:'postgres',configuration:{connection_info:{database_url:{from_env:'HASURA_GRAPHQL_DATABASE_URL'}}},tables:[]}]}})
    const table=yaml.load(read('../metadata/databases/backend-db/tables/sequent_backend_election_event.yaml'))
    const columns = new Set()
    for (const operation of ['select','insert','update']) for (const entry of table[operation+'_permissions']||[]) for (const col of entry.permission.columns||[]) if(col!=='administration_settings') columns.add(col)
    const type = name => name==='id'||name.endsWith('_id')?'uuid':name.startsWith('is_')?'boolean':name.endsWith('_at')?'timestamptz':['description','encryption_protocol','user_boards','public_key'].includes(name)?'text':'jsonb'
    const defs = [...columns].map(col => `"${col}" ${type(col)}${col==='id'?' PRIMARY KEY':''}`).join(',')
    await sql(`CREATE SCHEMA sequent_backend; CREATE TABLE sequent_backend.election_event(${defs}); CREATE TABLE sequent_backend.election(id uuid PRIMARY KEY,tenant_id uuid,election_event_id uuid,presentation jsonb); CREATE TABLE sequent_backend.election_lifecycle(tenant_id uuid,election_event_id uuid,phase text);`)
    await sql(read('../migrations/backend-db/1793020006000_administrator_settings/up.sql'))
    await sql(`INSERT INTO sequent_backend.election_event(id,tenant_id,is_archived,presentation,statistics) VALUES('${event}','${tenant}',false,'{"i18n":{"en":{"name":"Context","description":"ballot data"}},"otp":{"policy":"required"},"materials":{"ballot":"private"}}','{"votes":12}'); INSERT INTO sequent_backend.election_lifecycle VALUES('${tenant}','${event}','preparation');`)
    const selected = new Set(['admin-user','election-event-read','election-event-create','election-event-delete','election-event-write','service-account'])
    const metadata = {table:table.table}
    for(const operation of ['select','insert','update','delete']) metadata[operation+'_permissions']=(table[operation+'_permissions']||[]).filter(entry => selected.has(entry.role))
    await privileged('/v1/metadata',{type:'pg_track_table',args:{source:'default',table:table.table}})
    for(const operation of ['select','insert','update','delete']) for(const entry of metadata[operation+'_permissions']) await privileged('/v1/metadata',{type:'pg_create_'+operation+'_permission',args:{source:'default',table:table.table,role:entry.role,permission:entry.permission}})
    const projection=await query('{sequent_backend_election_event{id administration_settings}}')
    assert.deepEqual(projection.data.sequent_backend_election_event,[{id:event,administration_settings:{i18n:{en:{name:'Context'}},otp:{policy:'required'}}}])
    assert.deepEqual((await query('{sequent_backend_election_event{id}}',token(['admin-user'],otherTenant))).data.sequent_backend_election_event,[])
    denied(await query('{sequent_backend_election_event{presentation statistics status public_key}}'))
    denied(await query('{sequent_backend_election_event{presentation}}',admin,'election-event-read'))
    const allowed=await query('{sequent_backend_election_event{presentation statistics}}',board,'election-event-read')
    assert.equal(allowed.data.sequent_backend_election_event[0].statistics.votes,12)
    for(const mutation of [
      'mutation{delete_sequent_backend_election_event(where:{}){affected_rows}}',
      `mutation{insert_sequent_backend_election_event_one(object:{id:"${event}"}){id}}`,
      'mutation{update_sequent_backend_election_event(where:{},_set:{is_archived:true}){affected_rows}}',
      'mutation{update_sequent_backend_election_event(where:{},_set:{presentation:{dates:{start:"2030"}}}){affected_rows}}']) denied(await query(mutation))
    const save = 'mutation{update_sequent_backend_election_event(where:{},_set:{presentation:{otp:{policy:"stronger"}}}){affected_rows returning{administration_settings}}}'
    assert.equal((await query(save)).data.update_sequent_backend_election_event.affected_rows,1)
    const after=await query('{sequent_backend_election_event{presentation}}',board,'election-event-read')
    assert.equal(after.data.sequent_backend_election_event[0].presentation.materials.ballot,'private')
    await sql("UPDATE sequent_backend.election_lifecycle SET phase='execution'")
    denied(await query(save))
    // Match Rocket's (Status, String) responder: JSON text with text/plain.
    // Hasura must retain the typed signing requirement used by the portal.
    const pending={message:'Board authorisation required',extensions:{code:'signing-required',signing_request:{id:'fixture-request',required:2}}}
    const actionServer=http.createServer((req,res)=>{res.writeHead(409,{'content-type':'text/plain; charset=utf-8'});res.end(JSON.stringify(pending))})
    await new Promise(resolve=>actionServer.listen(18086,'127.0.0.1',resolve))
    try {
      await privileged('/v1/metadata',{type:'set_custom_types',args:{objects:[{name:'BoardFixtureOutput',fields:[{name:'ok',type:'Boolean!'}]}]}})
      await privileged('/v1/metadata',{type:'create_action',args:{name:'protected_fixture',definition:{kind:'synchronous',type:'mutation',handler:'http://127.0.0.1:18086',output_type:'BoardFixtureOutput'}}})
      await privileged('/v1/metadata',{type:'create_action_permission',args:{action:'protected_fixture',role:'admin-user'}})
      const response=await query('mutation{protected_fixture{ok}}')
      assert.equal(response.errors[0].extensions.code,'signing-required')
      assert.equal(response.errors[0].extensions.signing_request.id,'fixture-request')
    } finally { await new Promise(resolve=>actionServer.close(resolve)) }
    console.log('PASS: Hasura preserves the typed 409 board-signing request from the normal HTTP action response')
    console.log('PASS: authenticated Hasura administrator projection, tenant scope, raw field/role/insert/delete/archive denials, board read, permitted settings update and non-Preparation refusal')
}
main().catch(error=>{console.error(error);process.exitCode=1})
