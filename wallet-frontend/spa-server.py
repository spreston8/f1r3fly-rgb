#!/usr/bin/env python3
import os
import http.server
import socketserver
from pathlib import Path

class SPAHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP handler that serves index.html for all routes (SPA support)"""
    
    def do_GET(self):
        # Get the requested path
        path = self.translate_path(self.path)
        
        # If it's a file (not a directory) and exists, serve it normally
        if os.path.isfile(path):
            return super().do_GET()
        
        # If it's an asset (has extension), return 404
        if '.' in os.path.basename(self.path):
            return super().do_GET()
        
        # Otherwise, serve index.html (SPA routing)
        self.path = '/index.html'
        return super().do_GET()

PORT = 80
DIRECTORY = "/home/ubuntu/f1r3fly-rgb/wallet-frontend/dist"

os.chdir(DIRECTORY)

with socketserver.TCPServer(("", PORT), SPAHTTPRequestHandler) as httpd:
    print(f"Serving SPA from {DIRECTORY} at port {PORT}")
    httpd.serve_forever()