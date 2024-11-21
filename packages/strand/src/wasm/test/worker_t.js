/**
SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
SPDX-FileCopyrightText: 2022 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
*/
import "./fd.js";
import * as pkg from "../../../pkg_t/index.js";
pkg.default().then(_ => {
    var parameters = {}
    location.search.slice(1).split("&").forEach( function(key_value) { var kv = key_value.split("="); parameters[kv[0]] = kv[1]; });
    postMessage(`Initialized wasm`);
    postMessage('Cross origin isolated: ' + self.crossOriginIsolated);
    var go = function() {
        pkg.test();
        postMessage(' ');
        if(parameters['bench'] === 'true') {
            pkg.bench(10);
        }
        postMessage(' ');
        postMessage('ok');
    }
    self.onmessage = function(e) {
        console.log("In worker: " + JSON.stringify(e.data));
    };
    wasmFeatureDetect.threads().then(threads => {
        if (threads && pkg.initThreadPool) {
            postMessage('Thread pool supported, initThreadPool with conc = ' + navigator.hardwareConcurrency + '..');
            pkg.initThreadPool(navigator.hardwareConcurrency).then(_ => {
                postMessage('Thread pool initialized');
                go();
            })
        }
        else {
            postMessage('Thread pool NOT supported');
            go();    
        }
    });
})