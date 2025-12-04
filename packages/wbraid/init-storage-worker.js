// Web Worker to initialize SQLite OPFS VFS
// This must run in a Worker context because OPFS createSyncAccessHandle() requires it

console.log('[Worker] Storage initialization worker started');

self.onmessage = async (e) => {
    if (e.data.type === 'init-storage') {
        try {
            console.log('[Worker] Loading WASM module...');
            
            // Import the WASM module (use same cache-busting as main thread)
            const timestamp = e.data.timestamp || new Date().getTime();
            const { default: init, WasmSession } = await import(`./crates/braid/pkg/braid.js?v=${timestamp}`);
            
            console.log('[Worker] Initializing WASM...');
            await init({});
            
            console.log('[Worker] Calling init_storage()...');
            await WasmSession.init_storage();
            
            console.log('[Worker] Storage initialized successfully!');
            self.postMessage({ type: 'success' });
        } catch (err) {
            console.error('[Worker] Storage initialization failed:', err);
            self.postMessage({ 
                type: 'error', 
                error: err.message || String(err) 
            });
        }
    }
};
