# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import os
from http.server import HTTPServer, SimpleHTTPRequestHandler

class CrossOriginIsolation(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store, no-cache, must-revalidate')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        
        return super(CrossOriginIsolation, self).end_headers()

handler = CrossOriginIsolation
handler.extensions_map.update({
    '.wasm': 'application/wasm',
    '.js': 'text/javascript',
})

port = int(os.environ.get('PORT', '8080'))

print(f"Launching server on 127.0.0.1:{port}..")
print("SharedArrayBuffer support enabled (Cross-Origin-Isolation headers)")
httpd = HTTPServer(('127.0.0.1', port), handler)
httpd.serve_forever()
