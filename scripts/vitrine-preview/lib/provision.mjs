// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Writes into the stack's events log what the booth check needs for Step's
// election: its `vitrine/config/v1` statement (key, ballot styles and the
// portal's release) and the event's single `voting-opened` milestone naming
// it (Vitrine's specs/entry.md, sections 5.1 to 5.3), signed with the
// operator's authority key, as Step's Windmill writes them. An events log
// takes one voting-opened entry, so this works only on a log that has none
// yet; on a log whose milestone doesn't name the election it says so and
// writes nothing.

import fs from "node:fs";
import path from "node:path";
import { configFor, configInForce, readEntries, readEventsLog, receiptFor } from "./events.mjs";
import { ballotStyleText, styleDigests } from "./fixtures.mjs";
import { eventsEntry, leafHash, sha256, styleDigest, stylesHash } from "./vitrine.mjs";
import { readToml } from "./stack.mjs";

const WAIT_MS = 120000;

/** The demo booth's own configuration, so that a milestone written before the
 * booth sets itself up names its election too and the booth adopts it. */
export function demoBoothConfig({ live, vitrineDir }) {
  const booth = readToml(fs.readFileSync(path.join(live.stateDir, "booth", "config.toml"), "utf8"));
  const dir = path.join(vitrineDir, "packages", "demo-booth");
  const api = fs.readFileSync(path.join(dir, "src", "api.ts"), "utf8");
  // The booth names itself `<package> <version>`: the package from its RELEASE constant, the
  // version its build takes from package.json.
  const pkg = /RELEASE\s*=\s*\{[^}]*package:\s*"([^"]+)"/s.exec(api)?.[1];
  const version = JSON.parse(fs.readFileSync(path.join(dir, "package.json"), "utf8")).version;
  if (!pkg || !version) throw new Error("can't read the demo booth's release from packages/demo-booth");
  const styles = booth.setup.styles.map((s) => fs.readFileSync(path.join(dir, "styles", path.basename(s))));
  return {
    election: booth.election,
    key: sha256(Buffer.from(booth.setup.public_key_base64, "base64")),
    styles: stylesHash(styles.map((s) => styleDigest(s.toString("utf8")))),
    release: { package: pkg, version },
  };
}

export async function provisionEvents({ live, sidecar, electionId, release, dryRun, withDemoBooth, vitrineDir, log }) {
  const { entries, size } = await readEntries(live.events.url);
  const found = readEventsLog(entries);
  log(`the events log has ${size} entries: ${found.configs.length} config statements, ${found.opened.length} voting-opened milestones`);

  const text = ballotStyleText({ live, electionId, policyNote: live.policy });
  const style = styleDigests(text);
  const now = Math.floor(Date.now() / 1000);
  const statement = configFor({ live, electionId, key: style.key, styles: style.hash, release, time: now });

  const inForce = configInForce(found, live.eventId, electionId);
  if (inForce.config) {
    log(`already provisioned: the milestone names the configuration of ${electionId} (entry ${inForce.config.index}).`);
    const same = inForce.config.text.split("\n").filter((l) => /^(key|styles|release) /.test(l));
    const wanted = statement.split("\n").filter((l) => /^(key|styles|release) /.test(l));
    if (same.join("\n") !== wanted.join("\n")) {
      log("but it commits to a different key, ballot styles or release than the ones this preview serves:");
      log(`  log:     ${same.join(" | ")}`);
      log(`  preview: ${wanted.join(" | ")}`);
      log("the portal's booth check will refuse it. Serve the same release (--release-version) or re-provision the stack.");
      return false;
    }
    return true;
  }
  if (found.opened.length > 0) {
    log(`can't provision: ${inForce.reason}.`);
    log("An events log takes one voting-opened milestone (the rule one-opened); a second one would make the log fail it.");
    log("Re-provision the stack (deploy/down.sh, deploy/up.sh) and run this right after the log is up and before the demo booth sets itself up, or run it with --with-demo-booth.");
    return false;
  }

  const entriesToWrite = [];
  const configs = new Map();
  const add = (kind, statementText) => {
    const entry = eventsEntry({ kind, time: now, statement: live.signer.sign(statementText) });
    entriesToWrite.push({ kind, entry });
    return leafHash(entry);
  };
  configs.set(electionId, add("config", statement));
  if (withDemoBooth) {
    const booth = demoBoothConfig({ live, vitrineDir });
    const lines = configFor({ live, electionId: booth.election, key: booth.key, styles: booth.styles, release: booth.release, time: now });
    configs.set(booth.election, add("config", lines));
  }
  const milestone = `vitrine/milestone/v1\nevent ${live.eventId}\nkind voting-opened\ntime ${now}\n${[...configs].map(([e, leaf]) => `config ${e} ${leaf.toString("base64")}`).join("\n")}\n`;
  add("voting-opened", milestone);

  for (const { kind, entry } of entriesToWrite) log(`${dryRun ? "would write" : "writing"} ${kind} (${entry.length} bytes, leaf ${leafHash(entry).toString("hex").slice(0, 16)}...)`);
  if (dryRun) {
    log(`config statement for ${electionId}:\n${statement}`);
    return true;
  }
  for (const { kind, entry } of entriesToWrite) {
    const answer = await sidecar.add(live.events, entry, 5000, 8000);
    if (answer.status !== 200 && answer.status !== 202) throw new Error(`the events log refused the ${kind} (${answer.status}: ${answer.body.slice(0, 120)})`);
  }
  const last = leafHash(entriesToWrite.at(-1).entry);
  const end = Date.now() + WAIT_MS;
  while (Date.now() < end) {
    if (await receiptFor(live, last)) {
      log("provisioned: the milestone has a receipt that meets the quorum.");
      return true;
    }
    await new Promise((r) => setTimeout(r, 2000));
  }
  log("written, but no receipt that meets the quorum yet; the booth check passes once there is one");
  return true;
}
