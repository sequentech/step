/**
/* SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
*/
import "./fd.js";
import * as pkg from "../../../pkg/index.js";
pkg.default().then(_ => {
    var parameters = {};
    location.search.slice(1).split("&").forEach(
        function(key_value) {
            var kv = key_value.split("="); parameters[kv[0]] = kv[1];
        }
    );
    log(`Initialized wasm`);
    log('Cross origin isolated: ' + self.crossOriginIsolated);
    self.onmessage = function(e) {
        if(e.data.type == "encrypt") {
            var arg = e.data;
            console.log("Worker: encrypt called with: " + JSON.stringify(arg));
            var result = pkg.encrypt(arg.yes, arg.no);
            console.log("Worker: encrypt returns: " + JSON.stringify(result));
            postMessage({type: "encrypted", data: result});
        }
        else if(e.data.type == "shuffle") {
            var arg = e.data.data;
            console.log("Worker: shuffle called with: " + JSON.stringify(arg));
            log("Shuffling..");
            var result = pkg.shuffle(arg);
            console.log("Worker: shuffle returns: " + JSON.stringify(result));
            postMessage({type: "shuffled", data: result});
        }
        else if(e.data.type == "decrypt") {
            var arg = e.data.data;
            console.log("Worker: decrypt called with: " + JSON.stringify(arg));
            var result = pkg.decrypt(arg);
            console.log("Worker: decrypt returns: " + JSON.stringify(result));
            postMessage({type: "decrypted", data: result});
        }
    };
    wasmFeatureDetect.threads().then(threads => {
        if (threads && pkg.initThreadPool) {
            log('Thread pool supported, initThreadPool with conc = ' + (navigator.hardwareConcurrency) + '..');
            pkg.initThreadPool(navigator.hardwareConcurrency).then(_ => {
                log('Thread pool initialized');
                postMessage({type: "ready"});
            })
        }
        else {
            log('Thread pool NOT supported');
        }
    });
})

function log(log) {
    var message = {
        type: "log",
        data: log
    };
    postMessage(message);
}