/**
SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
SPDX-FileCopyrightText: 2022 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
*/
import * as pkg from "../../../pkg/index.js";
pkg.default().then(_ => {
    var parameters = {}
    location.search.slice(1).split("&").forEach( function(key_value) { var kv = key_value.split("="); parameters[kv[0]] = kv[1]; });
    postMessage(`Initialized wasm`);
    pkg.test();
    postMessage(' ');
    if(parameters['bench'] === 'true') {
        pkg.bench(10);
    }
    postMessage(' ');
    postMessage('ok');
})