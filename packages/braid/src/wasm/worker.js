import "./fd.js";
import * as pkg from '../../pkg_t/index.js';
pkg.default().then(_ => {
    postMessage(`Initialized wasm`);
    postMessage('Cross origin isolated: ' + self.crossOriginIsolated);
    var go = function() {
        var count = 500;
        postMessage('Running protocol test..');
        const t0 = performance.now();
        pkg.test_protocol_wasm(count, 2);
        const t1 = performance.now();
        postMessage(`Ok (${count}, ${(t1 - t0) / 1000.0}s)`);
        
    }
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
        }
    });
});