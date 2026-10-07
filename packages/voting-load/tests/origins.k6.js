// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import { check } from "k6";
import { Counter } from "k6/metrics";
import { approvedUrl, origin } from "../replay.k6.js";

const cases = new Counter("origin_cases");
export const options = {
  thresholds: { checks: ["rate==1"], origin_cases: ["count==12"] },
};

export default function () {
  for (const [url, expected] of [
    ["https://KEYCLOAK.example:443/realms/test", "https://keycloak.example"],
    ["http://keycloak.example:80/realms/test", "http://keycloak.example"],
    [
      "http://keycloak.example:8090/realms/test",
      "http://keycloak.example:8090",
    ],
    ["https://keycloak.example?x=1", "https://keycloak.example"],
    ["http://[::1]:80/", "http://[::1]"],
    ["https://[0:0:0:0:0:0:0:1]:443/realms/test", "https://[::1]"],
    ["https://bücher.example/realms/test", "https://xn--bcher-kva.example"],
    ["http://127.000.000.001/realms/test", "http://127.0.0.1"],
    ["not a URL", undefined],
    ["ftp://keycloak.example/", undefined],
    [
      "https://keycloak.example.attacker.test/",
      "https://keycloak.example.attacker.test",
    ],
    ["https://keycloak.example@attacker.test/", undefined],
  ]) {
    cases.add(1);
    check(origin(url), { "correct origin": (actual) => actual === expected });
    if (expected) {
      const canonical = approvedUrl(url, [expected]);
      check(canonical, {
        "request URL has canonical origin": (actual) =>
          actual.startsWith(expected + "/"),
      });
    }
    let blocked = false;
    try {
      approvedUrl(url, ["https://unapproved.example"]);
    } catch (error) {
      blocked = error.message === "Unapproved profile origin";
    }
    check(blocked, { "unapproved request rejected": (actual) => actual });
  }
}
