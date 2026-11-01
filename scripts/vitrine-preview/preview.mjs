#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Runs Step's voting and admin portals in a browser against the live local
// Vitrine stack, without a Step deployment. See README.md.
//
//   node preview.mjs start [flags]    start everything (returns once it is serving)
//   node preview.mjs stop             stop what `start` started, and nothing else
//   node preview.mjs status           what is running, and what the stack still needs
//   node preview.mjs tamper on|off    the Alert scenario's switch
//   node preview.mjs dishonest-portal on|off   the voting portal hides hubs' problems
//   node preview.mjs provision-events write the election's configuration into the events log
//   node preview.mjs settings voting|admin   print the /global-settings.json served

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createFront, releaseIdentity } from "./lib/front.mjs";
import { createStandIn } from "./lib/standin.mjs";
import { Sidecar } from "./lib/sidecar.mjs";
import { pickElection, readStack } from "./lib/stack.mjs";
import { createTamper } from "./lib/tamper.mjs";
import { createDishonestPortal } from "./lib/dishonest-portal.mjs";
import { provisionEvents } from "./lib/provision.mjs";
import { TENANT_ID } from "./lib/fixtures.mjs";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const HOME = os.homedir();

export const PORTS = { voting: 18850, standin: 18851, votingDev: 18852, admin: 18853, adminDev: 18854 };
const ORIGINS = { voting: `http://127.0.0.1:${PORTS.voting}`, standin: `http://127.0.0.1:${PORTS.standin}`, admin: `http://127.0.0.1:${PORTS.admin}` };
const RELAY_CONTAINER = "vitrine-ui-harness-relay";
const PLAYWRIGHT_IMAGE = "mcr.microsoft.com/playwright:v1.64.1-noble";
const MAX_SOCKET_PATH = 100;

function parseArgs(argv) {
  const flags = {};
  const rest = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const [k, v] = a.slice(2).split("=");
      if (v !== undefined) flags[k] = v;
      else if (argv[i + 1] !== undefined && !argv[i + 1].startsWith("--") && !["wait", "no-admin", "no-voting", "dry-run", "with-demo-booth"].includes(k)) flags[k] = argv[++i];
      else flags[k] = true;
    } else rest.push(a);
  }
  return { flags, rest };
}

function config(flags) {
  const vitrine = path.resolve(flags.vitrine ?? process.env.VITRINE_DIR ?? path.join(HOME, "vitrine"));
  const step = path.resolve(flags.step ?? process.env.STEP_DIR ?? path.join(HOME, "step-vitrine"));
  const runtime = path.resolve(flags.runtime ?? path.join(HERE, "run"));
  const logs = path.resolve(flags.logs ?? process.env.PREVIEW_LOGS ?? path.join(runtime, "logs"));
  let socketDir = path.join(runtime, "relay");
  if (path.join(socketDir, "internal.sock").length > MAX_SOCKET_PATH) socketDir = path.join(os.tmpdir(), `vitrine-preview-${process.getuid()}`, "relay");
  const dist = flags["static-dist"] ? path.resolve(flags["static-dist"]) : undefined;
  const releaseFile = flags["release-bundle"] ? path.resolve(flags["release-bundle"]) : undefined;
  return {
    vitrine,
    step,
    packages: path.join(step, "packages"),
    runtime,
    logs,
    socketDir,
    socket: path.join(socketDir, "internal.sock"),
    dist,
    releaseFile,
    releasePackage: flags["release-package"],
    releaseVersion: flags["release-version"],
    version: flags.version,
    election: flags.election,
    netContainer: flags["net-container"] ?? "vitrine-recovery-dev-net",
    only: flags["no-admin"] ? "voting" : flags["no-voting"] ? "admin" : "both",
    wait: Boolean(flags.wait),
  };
}

const sh = (cmd, args, opts = {}) => spawnSync(cmd, args, { encoding: "utf8", ...opts });
const pidsFile = (c) => path.join(c.runtime, "pids.json");
const readPids = (c) => {
  try {
    return JSON.parse(fs.readFileSync(pidsFile(c), "utf8"));
  } catch {
    return {};
  }
};
const alive = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

/** The release the config statement and the portal's own build name. */
function releaseOf(c) {
  const version = c.releaseVersion ?? c.version ?? "dev";
  const fallback = { package: c.releasePackage ?? "sequent/voting-portal", version };
  if (!c.releaseFile) return { ...fallback, file: undefined };
  const named = releaseIdentity(c.releaseFile, fallback);
  return { package: c.releasePackage ?? named.package, version: c.releaseVersion ?? named.version, file: c.releaseFile };
}

function portalSettings(c, live, kind, release) {
  const common = {
    DEFAULT_TENANT_ID: TENANT_ID,
    KEYCLOAK_URL: `${ORIGINS.standin}/`,
    HASURA_URL: `${kind === "voting" ? ORIGINS.voting : ORIGINS.standin}/v1/graphql`,
    APP_VERSION: release.version,
    APP_HASH: release.version,
    PUBLIC_BUCKET_URL: `${ORIGINS.standin}/public/`,
    KIOSK_VOTING_PORTAL_URL: "",
  };
  if (kind === "voting") {
    return {
      DISABLE_AUTH: false,
      QUERY_POLL_INTERVAL_MS: 2000,
      ...common,
      DEFAULT_EVENT_ID: live.eventId,
      ONLINE_VOTING_CLIENT_ID: "voting-portal",
      KIOSK_KEYCLOAK_URL: "",
      BALLOT_VERIFIER_URL: live.stack.apps?.verifier ?? "",
      RESULTS_PORTAL_URL: "",
      KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: 900,
      POLLING_DURATION_TIMEOUT: 12000,
    };
  }
  return {
    QUERY_POLL_INTERVAL_MS: 30000,
    ...common,
    ONLINE_VOTING_CLIENT_ID: "admin-portal",
    VOTING_PORTAL_URL: ORIGINS.voting,
    RESULTS_PORTAL_URL: "",
    ACTIVATE_MIRU_EXPORT: false,
    DEFAULT_EMAIL_SUBJECT: { en: "Participate in {{election_event.name}}" },
    DEFAULT_EMAIL_HTML_BODY: { en: "<p>Hello {{user.first_name}},<br><br>Enter in {{vote_url}} to vote</p>" },
    DEFAULT_EMAIL_PLAINTEXT_BODY: { en: "Hello {{user.first_name}},\n\nEnter in {{vote_url}} to vote" },
    DEFAULT_SMS_MESSAGE: { en: "Enter in {{vote_url}} to vote" },
  };
}

// -- The relay container -------------------------------------------------------------

function startRelay(c) {
  sh("docker", ["rm", "-f", RELAY_CONTAINER]);
  fs.mkdirSync(c.socketDir, { recursive: true });
  fs.rmSync(c.socket, { force: true });
  const run = sh("docker", [
    "run", "-d", "--name", RELAY_CONTAINER, "--network", `container:${c.netContainer}`,
    "--user", `${process.getuid()}:${process.getgid()}`, "-e", "HOME=/tmp",
    "-v", `${c.socketDir}:/relay`, "-v", `${path.join(HERE, "relay.mjs")}:/relay.mjs:ro`,
    "--entrypoint", "node", PLAYWRIGHT_IMAGE, "/relay.mjs", "/relay/internal.sock", "127.0.0.1", "28710",
  ]);
  if (run.status !== 0) throw new Error(`couldn't start the relay container: ${run.stderr.trim()}`);
}

// -- start -----------------------------------------------------------------------------

function spawnDetached(c, name, cmd, args, env = {}, cwd) {
  fs.mkdirSync(c.logs, { recursive: true });
  const out = fs.openSync(path.join(c.logs, `${name}.log`), "a");
  const child = spawn(cmd, args, { cwd, env: { ...process.env, ...env }, detached: true, stdio: ["ignore", out, out] });
  child.unref();
  return child.pid;
}

async function waitFor(what, test, timeoutMs, everyMs = 1000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    try {
      const ok = await test();
      if (ok) return true;
    } catch (e) {
      last = e;
    }
    await new Promise((r) => setTimeout(r, everyMs));
  }
  throw new Error(`${what} didn't come up (${last?.message ?? "no answer"})`);
}

const reachable = async (url) => {
  const res = await fetch(url, { signal: AbortSignal.timeout(4000) });
  return res.status < 500;
};

async function start(c) {
  fs.mkdirSync(c.runtime, { recursive: true });
  const pids = readPids(c);
  if (Object.values(pids).some((p) => typeof p === "number" && alive(p))) throw new Error("the preview is already running; stop it first");
  if (!fs.existsSync(path.join(c.packages, "voting-portal/webpack.config.cjs"))) throw new Error(`no Step checkout at ${c.step}`);
  const live = readStack(c.vitrine);
  await reachable(`${live.stack.policy_url}`).catch(() => {
    throw new Error("the Vitrine stack isn't answering at its policy URL; start it with deploy/up.sh");
  });
  const election = pickElection(live, c.election);
  startRelay(c);
  const sidecar = new Sidecar(live, c.socket);
  await waitFor("the relay to the log server", async () => (await sidecar.adminLogs()).length > 0, 20000);

  const env = { PREVIEW_CONFIG: JSON.stringify({ ...c, election }) };
  const next = { relay: RELAY_CONTAINER, config: { ...c, election } };
  next.serve = spawnDetached(c, "standin", process.execPath, [path.join(HERE, "preview.mjs"), "serve"], env);
  await waitFor("the stand-in", () => reachable(`${ORIGINS.standin}/healthz`), 20000, 500);

  const release = releaseOf(c);
  const webpack = path.join(c.packages, "node_modules/.bin/webpack");
  const nodeOptions = "--openssl-legacy-provider --max-old-space-size=4096";
  if (c.only !== "admin" && !c.dist) {
    next.votingDev = spawnDetached(c, "voting-portal", webpack, ["serve", "--config", "webpack.config.cjs", "--mode", "development", "--port", String(PORTS.votingDev), "--host", "127.0.0.1", "--no-open", "--no-client-overlay-warnings"], { NODE_OPTIONS: nodeOptions, PORT: String(PORTS.votingDev) }, path.join(c.packages, "voting-portal"));
  }
  if (c.only !== "voting") {
    next.adminDev = spawnDetached(c, "admin-portal", webpack, ["serve", "--config", "webpack.config.cjs", "--mode", "development", "--port", String(PORTS.adminDev), "--host", "127.0.0.1", "--no-open", "--no-client-overlay-warnings"], { NODE_OPTIONS: nodeOptions, PORT: String(PORTS.adminDev) }, path.join(c.packages, "admin-portal"));
  }
  fs.writeFileSync(pidsFile(c), JSON.stringify(next, null, 2));
  console.log(`stand-in:       ${ORIGINS.standin}  (a local stand-in for Step's backend, with a fake Keycloak)`);
  if (c.only !== "admin") console.log(`voting portal:  ${ORIGINS.voting}/  ${c.dist ? `(static build of ${c.dist}, release ${release.package} ${release.version})` : "(webpack dev server, compiling)"}`);
  if (c.only !== "voting") console.log(`admin portal:   ${ORIGINS.admin}/  (webpack dev server, compiling)`);
  console.log(`election:       ${live.eventId} / ${election}   policy ${live.policyId}`);
  console.log(`logs:           ${c.logs}`);
  if (c.wait) {
    const targets = [];
    if (c.only !== "admin") targets.push(`${ORIGINS.voting}/`);
    if (c.only !== "voting") targets.push(`${ORIGINS.admin}/`);
    for (const t of targets) await waitFor(`${t}`, async () => (await fetch(t, { signal: AbortSignal.timeout(8000) })).status === 200, 15 * 60 * 1000, 3000);
    console.log("the portals are compiled and serving");
  } else {
    console.log("the dev servers compile for a few minutes: `status` shows when they answer (or pass --wait)");
  }
}

// -- serve (the one process hosting the stand-in and the fronts) -------------------------

async function serve() {
  const c = JSON.parse(process.env.PREVIEW_CONFIG);
  const logFile = (name) => (line) => console.log(`${new Date().toISOString()} ${name} ${line}`);
  const log = logFile("standin");
  const live = readStack(c.vitrine);
  const sidecar = new Sidecar(live, c.socket);
  const release = releaseOf(c);
  const tamper = createTamper({ live, log });
  const dishonest = createDishonestPortal({ log });
  const standin = createStandIn({
    live, sidecar, origin: ORIGINS.standin, stepPackages: c.packages, electionId: c.election, version: release.version,
    voterOrigin: ORIGINS.voting, adminOrigin: ORIGINS.admin, tamper, dishonest, log,
  });
  const listen = (server, port) => new Promise((resolve, reject) => server.once("error", reject).listen(port, "127.0.0.1", resolve));
  await listen(standin.server, PORTS.standin);
  if (c.only !== "admin") {
    const front = createFront({ settings: portalSettings(c, live, "voting", release), upstreamPort: PORTS.votingDev, graphqlPort: PORTS.standin, dist: c.dist, release: c.releaseFile ? release : undefined, dishonest, log: logFile("voting") });
    await listen(front.server, PORTS.voting);
  }
  if (c.only !== "voting") {
    const front = createFront({ settings: portalSettings(c, live, "admin", release), upstreamPort: PORTS.adminDev, log: logFile("admin") });
    await listen(front.server, PORTS.admin);
  }
  log(`serving: stand-in :${PORTS.standin}, voting :${PORTS.voting}, admin :${PORTS.admin}; election ${live.eventId}/${c.election}`);
}

// -- stop ----------------------------------------------------------------------------------

async function stop(c) {
  const pids = readPids(c);
  for (const [name, pid] of Object.entries(pids)) {
    if (typeof pid !== "number" || !alive(pid)) continue;
    try {
      process.kill(-pid, "SIGTERM");
    } catch {
      try {
        process.kill(pid, "SIGTERM");
      } catch {
        // Already gone.
      }
    }
    console.log(`stopped ${name} (${pid})`);
  }
  await new Promise((r) => setTimeout(r, 1500));
  for (const pid of Object.values(pids)) {
    if (typeof pid === "number" && alive(pid)) {
      try {
        process.kill(-pid, "SIGKILL");
      } catch {
        // Already gone.
      }
    }
  }
  if (pids.relay) {
    sh("docker", ["rm", "-f", RELAY_CONTAINER]);
    console.log(`removed the container ${RELAY_CONTAINER}`);
  }
  fs.rmSync(pidsFile(c), { force: true });
}

/** Restarts the process that hosts the stand-in and the fronts (after editing
 * the harness), leaving the dev servers compiled. */
async function reload(c) {
  const pids = readPids(c);
  if (!pids.config) throw new Error("the preview isn't running");
  if (typeof pids.serve === "number" && alive(pids.serve)) {
    process.kill(-pids.serve, "SIGTERM");
    await waitFor("the old stand-in to stop", async () => !alive(pids.serve), 10000, 200);
  }
  const saved = { ...pids.config, ...Object.fromEntries(Object.entries(c).filter(([k, v]) => v !== undefined && ["logs", "dist", "releaseFile", "releasePackage", "releaseVersion", "version"].includes(k))) };
  pids.serve = spawnDetached(saved, "standin", process.execPath, [path.join(HERE, "preview.mjs"), "serve"], { PREVIEW_CONFIG: JSON.stringify(saved) });
  fs.writeFileSync(pidsFile(c), JSON.stringify(pids, null, 2));
  await waitFor("the stand-in", () => reachable(`${ORIGINS.standin}/healthz`), 20000, 500);
  console.log("reloaded the stand-in and the fronts");
}

// -- status ----------------------------------------------------------------------------------

async function status(c) {
  const pids = readPids(c);
  for (const [name, pid] of Object.entries(pids)) console.log(`${name.padEnd(10)} ${typeof pid === "number" ? (alive(pid) ? `running (${pid})` : "not running") : pid}`);
  const probe = async (name, url) => {
    try {
      const res = await fetch(url, { signal: AbortSignal.timeout(5000) });
      console.log(`${name.padEnd(10)} ${url} -> ${res.status}`);
    } catch (e) {
      console.log(`${name.padEnd(10)} ${url} -> ${e.cause?.code ?? e.message}`);
    }
  };
  await probe("stand-in", `${ORIGINS.standin}/healthz`);
  await probe("voting", `${ORIGINS.voting}/`);
  await probe("admin", `${ORIGINS.admin}/`);
  try {
    const s = await (await fetch(`${ORIGINS.standin}/__preview/status`)).json();
    console.log(JSON.stringify(s, null, 2));
  } catch {
    // The stand-in isn't up.
  }
}

// -- main ---------------------------------------------------------------------------------------

const { flags, rest } = parseArgs(process.argv.slice(2));
const command = rest[0];
const c = config(flags);

try {
  if (command === "start") await start(c);
  else if (command === "stop") await stop(c);
  else if (command === "status") await status(c);
  else if (command === "reload") await reload(c);
  else if (command === "serve") await serve();
  else if (command === "tamper" || command === "dishonest-portal") {
    const mode = rest[1] ?? "status";
    const res = await fetch(`${ORIGINS.standin}/__preview/${command}`, { method: mode === "status" ? "GET" : "POST", headers: { "Content-Type": "application/json" }, body: mode === "status" ? undefined : JSON.stringify({ on: mode === "on" }) });
    console.log(JSON.stringify(await res.json(), null, 2));
  } else if (command === "provision-events") {
    const live = readStack(c.vitrine);
    startRelay(c);
    const sidecar = new Sidecar(live, c.socket);
    await waitFor("the relay to the log server", async () => (await sidecar.adminLogs()).length > 0, 20000);
    try {
      await provisionEvents({ live, sidecar, electionId: pickElection(live, c.election), release: releaseOf(c), dryRun: Boolean(flags["dry-run"]), withDemoBooth: Boolean(flags["with-demo-booth"]), vitrineDir: c.vitrine, log: console.log });
    } finally {
      if (!readPids(c).relay) sh("docker", ["rm", "-f", RELAY_CONTAINER]);
    }
  } else if (command === "settings") {
    const live = readStack(c.vitrine);
    process.stdout.write(`${JSON.stringify(portalSettings(c, live, rest[1] === "admin" ? "admin" : "voting", releaseOf(c)), null, 2)}\n`);
  } else {
    console.error("usage: preview.mjs start|stop|status|tamper on|off|dishonest-portal on|off|provision-events|settings voting|admin  [--static-dist DIR] [--release-bundle FILE] [--version V] [--wait]");
    process.exitCode = 2;
  }
} catch (e) {
  console.error(e.message);
  process.exitCode = 1;
}
