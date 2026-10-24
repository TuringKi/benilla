#!/usr/bin/env python3
"""Receive bounded, UI-only browser diagnostic reports on loopback."""
import json
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LOG = Path(__file__).with_name('ui-browser-reports.jsonl')

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        size = int(self.headers.get('Content-Length', '0'))
        if self.path != '/ui-report' or not 0 < size <= 65536:
            self.send_error(400)
            return
        try:
            report = json.loads(self.rfile.read(size))
            report = {k: v for k, v in report.items()
                      if k in ('revision', 'logs', 'frames', 'phase', 'gpu')}
            report['time'] = datetime.now(timezone.utc).isoformat()
            with LOG.open('a') as out:
                out.write(json.dumps(report, ensure_ascii=False) + '\n')
        except (ValueError, AttributeError):
            self.send_error(400)
            return
        self.send_response(204)
        self.end_headers()

ThreadingHTTPServer(('127.0.0.1', 8092), Handler).serve_forever()
