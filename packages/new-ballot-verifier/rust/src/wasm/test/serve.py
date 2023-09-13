# SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
# SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

from http.server import HTTPServer, SimpleHTTPRequestHandler

class CrossOriginIsolation(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store, no-cache, must-revalidate')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        
        return super(CrossOriginIsolation, self).end_headers()

handler = CrossOriginIsolation
handler.extensions_map.update({
    '.wasm': 'application/wasm'
})
httpd = HTTPServer(('localhost', 8080), handler)
httpd.serve_forever()
