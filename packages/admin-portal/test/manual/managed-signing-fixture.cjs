// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const assert = require("node:assert/strict")
const fs = require("node:fs")
const os = require("node:os")
const {execFileSync} = require("node:child_process")
const https = require("node:https")
const path = require("node:path")
const src = path.resolve(__dirname, "../../src")
const {chromium} = require("playwright")
const esbuild = require("esbuild")
const source = `
import React from "react"
import {createRoot} from "react-dom/client"
import {OperationalApprovalDialog} from "${src}/components/signing/OperationalApprovalDialog"
import {canonicalJson} from "${src}/lib/signing/request"
import {hex, sha256} from "${src}/lib/signing/der"
import {AuthContext} from "@/providers/AuthContextProvider"
import {installClient} from "@apollo/client"
const id=i=>"55555555-5555-4555-8555-55555555555"+i
const scope={tenant:id(1),event:id(2),request:id(3),official:id(4)}
const post = id(5)
window.count={setup:0,starts:0,cancels:0,mutations:0,refresh:0,reauth:0}
window.challenge=null
window.consumed=false
window.activeOfficial=scope.official
window.setupPhase=sessionStorage.getItem("synthetic-retry")?"setup-retry-available":"setup-available"
window.ready=false
window.gold=!sessionStorage.getItem("synthetic-retry")
window.provider=sessionStorage.getItem("synthetic-retry")?"yubihsm":"softhsm-test"
window.tamper=false
const panel={operational_signing_mode:"hsm-required", count:0, signers:[{user_id:scope.official,username:"test",display_name:"Test",signed_at:null}],
  rule:{requester_signing:"allowed"},document_url:null}
const request={id:scope.request,tenant_id:scope.tenant,election_event_id:scope.event,election_id:post,action:"initialize-voting",status:"waiting",expires_at:null,
  document_sha256:null, required:2,code:"TEST",requested_by:scope.official,created_at:"2026-11-24T00:00:00Z",subject:{},area_id:null}
const bytes=canonicalJson({domain:"step-signing/v1",action:request.action,area_id:null,code:request.code,created_at:request.created_at,expires_at:null,
 election_id:post,election_event_id:scope.event,request_id:scope.request,requested_by:scope.official,tenant_id:scope.tenant,subject:{}})
const digest=hex(await sha256(new TextEncoder().encode(bytes)))
panel.request={...request,canonical_payload:bytes,payload_sha256:digest}
installClient({
  query:async({query})=>{
    if(window.failStatus) throw new Error("synthetic status unavailable")
    const activeScope={...scope,official:window.activeOfficial}
    return  query.startsWith("OperationalInitializeSetupStatus")?
    {data:{operationalInitializeSetupStatus:{scope:activeScope,phase:window.ready?"certificate-on-file":window.setupPhase}}}:
    {data:{operationalInitializeStatus:{scope:activeScope,phase:window.consumed?"pending":window.challenge?"waiting-for-identity":window.ready?"challenge-available":"unavailable",device_profile:window.provider,cancellable_attempt:window.consumed?null:window.challenge}}}
  },
  mutate:async({mutation,variables})=>{
    window.count.mutations++
    if(mutation.startsWith("OperationalInitializeSetup")) {
      window.count.setup++
      if(window.holdSetup) await new Promise(resolve=>{window.releaseSetup=resolve})
      window.ready=true
      return {data:{operationalInitializeSetup:{scope,phase:"certificate-on-file"}}}
    }
    if(mutation.startsWith("OperationalInitializeCancel")) {
      if(window.consumed || variables.attempt!==window.challenge) throw new Error("stale cancellation")
      window.count.cancels++
      window.challenge=null
      return {data:{operationalInitializeCancel:{phase:"challenge-available"}}}
    }
    if(mutation.startsWith("OperationalInitializeReconcile")) {
      window.count.operatingCheck=(window.count.operatingCheck ?? 0)+1
      if(window.holdCheck) await new Promise(resolve=>{window.releaseCheck=resolve})
      return {}
    }
    window.startVars=variables
    window.count.starts++
    window.challenge=id(6 + window.count.starts)
    throw new Error("synthetic HSM unavailable after attempt was reserved")
  }
})
function Fixture(){
 const [official,setOfficial]=React.useState(scope.official)
 const [visible,setVisible]=React.useState(true)
 window.reopen=()=>setVisible(true)
 window.switchActor=()=>{window.activeOfficial=id(9);setOfficial(id(9))}
 return <AuthContext.Provider value={{isAuthenticated:true,tenantId:scope.tenant,userId:official,isGoldUser:()=>window.gold,reauthWithGold:async(uri,fresh)=>{window.count.reauth++;window.goldArgs={uri,fresh}}}}>
 {visible&&<OperationalApprovalDialog data={panel} api={{getRequest:async()=>window.tamper ? {...panel,request:{...panel.request,election_id:id(9)}} : panel}} eligible
 onClose={()=>setVisible(false)} onRefresh={()=>window.count.refresh++}/>}
 </AuthContext.Provider>
}
createRoot(document.getElementById("root")).render(<Fixture/>)
`
const stubs = {
    "@/providers/AuthContextProvider":
        'import React from "react"; export const AuthContext=React.createContext({});',
    "@/providers/SettingsContextProvider":
        'import React from "react"; export const SettingsContext=React.createContext({globalSettings:{KEYCLOAK_URL:"https://localhost"}});',
    "@apollo/client":
        'let client; export const useApolloClient=()=>client; export const installClient=c=>client=c; export const gql=(s)=>s[0].trim().split(/\\s+/)[1].split("(")[0];',
    "react-i18next":
        'const text={setup:"Set up signing",approve:"Approve",softwareDemo:"Software token, not hardware",quorum:"0 of 2 approvals recorded"}; export const useTranslation=()=>({t:key=>text[key.split(".").at(-1)]||key});',
    "./SigningSubject":
        'import React from "react"; export const SigningSubjectVariant={Dialog:"dialog"}; export const SigningSubject=()=>React.createElement("div",null,"[synthetic signing subject]");',
}
;(async () => {
    const build = await esbuild.build({
        stdin: {contents: source, loader: "tsx", resolveDir: src},
        bundle: true,
        write: false,
        format: "esm",
        platform: "browser",
        tsconfig: path.resolve(src, "../tsconfig.json"),
        plugins: [
            {
                name: "fixture-boundaries",
                setup(b) {
                    b.onResolve({filter: /.*/}, (a) =>
                        Object.hasOwn(stubs, a.path) ? {path: a.path, namespace: "mock"} : null
                    )
                    b.onLoad({filter: /.*/, namespace: "mock"}, (a) => ({
                        contents: stubs[a.path],
                        resolveDir: src,
                    }))
                },
            },
        ],
    })
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), "step-managed-signing-fixture-"))
    let key, cert
    try {
        execFileSync(
            "openssl",
            [
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                "key.pem",
                "-out",
                "cert.pem",
                "-days",
                "1",
                "-subj",
                "/CN=localhost",
            ],
            {cwd: temp, stdio: "ignore"}
        )
        key = fs.readFileSync(path.join(temp, "key.pem"))
        cert = fs.readFileSync(path.join(temp, "cert.pem"))
    } finally {
        fs.rmSync(temp, {recursive: true, force: true})
    }
    const server = https.createServer({key, cert}, (req, res) => {
        if (req.url === "/app.js") {
            res.setHeader("Content-Type", "application/javascript")
            res.end(build.outputFiles[0].text)
        } else {
            res.setHeader("Content-Type", "text/html")
            res.end(
                '<meta charset=utf-8><div id=root></div><script src="/app.js" type="module"></script>'
            )
        }
    })
    await new Promise((r) => server.listen(0, "127.0.0.1", r))
    let browser
    try {
        browser = await chromium.launch({
            executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
            headless: true,
        })
        const page = await browser.newPage({ignoreHTTPSErrors: true})
        page.setDefaultTimeout(5000)
        page.on("pageerror", (err) => console.error("pageerror", err.message))
        const url =
            "https://127.0.0.1:" +
            server.address().port +
            "/sequent_backend_election/55555555-5555-4555-8555-555555555555/show"
        await page.goto(url)
        await page.getByRole("dialog").waitFor()
        await page.getByText("Software token, not hardware").waitFor()
        assert.equal(await page.locator('input[type="file"]').count(), 0)
        assert.equal(await page.getByRole("button", {name: "Approve", exact: true}).count(), 0)
        await page.evaluate(() => (window.gold = false))
        await page.getByRole("button", {name: "Set up signing", exact: true}).click()
        await page.waitForFunction(() => window.count.reauth === 1)
        assert.equal(await page.evaluate(() => window.count.setup), 0)
        assert.equal(await page.evaluate(() => window.goldArgs.fresh), true)
        await page.evaluate(() => (window.gold = true))
        await page.getByRole("button", {name: "Set up signing", exact: true}).click()
        await page.getByRole("button", {name: "Approve", exact: true}).waitFor()
        assert.equal(await page.evaluate(() => window.count.setup), 1)
        assert.equal(await page.evaluate(() => window.count.starts), 0)
        await page.getByText("0 of 2 approvals recorded").waitFor()
        await page.evaluate(() => (window.tamper = true))
        await page.getByRole("button", {name: "Approve", exact: true}).click()
        await page.getByText("signing.operational.error").waitFor()
        assert.equal(await page.evaluate(() => window.count.starts), 0)
        await page.evaluate(() => (window.tamper = false))
        await page.getByRole("button", {name: "Approve", exact: true}).click()
        await page.getByText("signing.operational.phase.waiting-for-identity").waitFor()
        assert.equal(await page.getByRole("button", {name: "Approve", exact: true}).count(), 0)
        assert.equal(await page.evaluate(() => window.count.starts), 1)
        const start = await page.evaluate(() => window.startVars)
        assert.equal(start.request, "55555555-5555-4555-8555-555555555553")
        assert.equal(start.page.election_id, "55555555-5555-4555-8555-555555555555")
        assert.match(start.digest, /^[a-f0-9]{64}$/)
        await page
            .getByRole("button", {name: "signing.operational.cancelUnused", exact: true})
            .click()
        assert.equal(await page.evaluate(() => window.count.cancels), 0)
        await page
            .getByRole("button", {name: "signing.operational.confirmDiscard", exact: true})
            .click()
        await page.getByRole("button", {name: "Approve", exact: true}).waitFor()
        assert.equal(await page.evaluate(() => window.count.cancels), 1)
        assert.equal(await page.evaluate(() => window.count.starts), 1)
        await page.getByRole("button", {name: "Approve", exact: true}).click()
        await page
            .getByRole("button", {name: "signing.operational.cancelUnused", exact: true})
            .click()
        // The challenge was consumed while the confirmation was open.
        await page.evaluate(() => {
            window.consumed = true
        })
        await page
            .getByRole("button", {name: "signing.operational.confirmDiscard", exact: true})
            .click()
        await page.getByText("signing.operational.phase.pending").waitFor()
        assert.equal(await page.evaluate(() => window.count.cancels), 1)
        assert.equal(await page.evaluate(() => window.count.starts), 2)
        assert.equal(await page.getByRole("button", {name: "Approve", exact: true}).count(), 0)
        assert.equal(
            await page
                .getByRole("button", {name: "signing.operational.cancelUnused", exact: true})
                .count(),
            0
        )

        await page.evaluate(() => sessionStorage.setItem("synthetic-retry", "1"))
        await page.reload()
        await page.getByText("signing.operational.isolatedDemo").waitFor()
        const retry = page.getByRole("button", {
            name: "signing.operational.retrySetup",
            exact: true,
        })
        await retry.waitFor()
        assert.equal(await page.getByText("Software token, not hardware").count(), 0)
        assert.equal(await page.evaluate(() => window.count.setup), 0)
        await retry.click()
        await page.waitForFunction(() => window.count.reauth === 1)
        assert.equal(await page.evaluate(() => window.count.setup), 0)
        await page.evaluate(() => {
            window.setupPhase = "setup-recovery-required"
            window.gold = true
        })
        await retry.click()
        await page.getByText("signing.operational.phase.setup-recovery-required").waitFor()
        assert.equal(await page.evaluate(() => window.count.setup), 0)
        assert.equal(await page.evaluate(() => window.count.starts), 0)
        assert.equal(await retry.count(), 0)
        assert.equal(await page.getByRole("button", {name: "Approve", exact: true}).count(), 0)
        await page.evaluate(() => {
            window.setupPhase = "setup-retry-available"
        })
        await page
            .getByRole("button", {name: "signing.operational.checkStatus", exact: true})
            .click()
        await retry.click()
        await page.getByRole("button", {name: "Approve", exact: true}).waitFor()
        assert.equal(await page.evaluate(() => window.count.setup), 1)
        assert.equal(await page.evaluate(() => window.count.starts), 0)
        await page.evaluate(() => sessionStorage.removeItem("synthetic-retry"))
        await page.reload()
        await page.getByText("Software token, not hardware").waitFor()
        await page.evaluate(() => {
            window.holdSetup = true
        })
        await page.getByRole("button", {name: "Set up signing", exact: true}).click()
        await page.waitForFunction(() => typeof window.releaseSetup === "function")
        await page.evaluate(() => {
            window.consumed = true
            window.provider = "yubihsm"
            window.holdCheck = true
            window.switchActor()
        })
        await page.getByText("signing.operational.phase.pending").waitFor()
        assert.equal(await page.getByText("Software token, not hardware").count(), 0)
        const close = page.getByRole("button", {name: "signing.widget.close", exact: true})
        const check = page.getByRole("button", {
            name: "signing.operational.checkStatus",
            exact: true,
        })
        assert.equal(await close.isEnabled(), true)
        await check.click()
        await page.waitForFunction(() => typeof window.releaseCheck === "function")
        assert.equal(await close.isEnabled(), false)
        await page.evaluate(() => window.releaseSetup())
        await page.waitForFunction(() => window.ready)
        assert.equal(await close.isEnabled(), false)
        await page.evaluate(() => window.releaseCheck())
        await page.waitForFunction(() => !document.querySelector("button:disabled"))
        assert.equal(await close.isEnabled(), true)
        assert.equal(await page.evaluate(() => window.count.starts), 0)
        assert.equal(await page.evaluate(() => window.count.setup), 1)
        await page.evaluate(() => {
            window.holdCheck = false
            window.provider = "softhsm-test"
        })
        await check.click()
        await page.getByText("Software token, not hardware").waitFor()
        await page.evaluate(() => {
            window.failStatus = true
        })
        await check.click()
        await page.getByText("signing.operational.phase.unavailable").waitFor()
        assert.equal(await page.getByText("Software token, not hardware").count(), 0)
        await page.getByText("signing.operational.isolatedDemo").waitFor()
        await close.click()
        await page.getByRole("dialog").waitFor({state: "detached"})
        await page.evaluate(() => {
            window.failStatus = false
            window.provider = "yubihsm"
            window.reopen()
        })
        await page.getByText("signing.operational.phase.pending").waitFor()
        assert.equal(await page.getByText("Software token, not hardware").count(), 0)
        assert.equal(await page.evaluate(() => window.count.starts), 0)
        assert.equal(await page.evaluate(() => window.count.setup), 1)
        console.log(
            "PASS synthetic adapter, real Chromium dialog: no upload; forced first-use Gold before setup; separate review/approval; wrong Post never starts; exact Post and digest sent; abandoned step-up can be explicitly discarded; consumed step-up cannot; stale provider/operation state cannot affect reopen or a newer operation; no duplicate approval."
        )
    } finally {
        await browser?.close()
        await new Promise((r) => server.close(r))
    }
})().catch((e) => {
    console.error(e)
    process.exit(1)
})
