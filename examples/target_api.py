"""Демо-API со специальными ошибками, которые должен найти apifuzz."""

import json
import os
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse

PORT = int(os.environ.get("PORT", "9101"))


def reply(handler, status, payload):
    data = json.dumps(payload).encode("utf-8")
    handler.send_response(status)
    handler.send_header("Content-Type", "application/json")
    handler.send_header("Content-Length", str(len(data)))
    handler.end_headers()
    handler.wfile.write(data)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path.startswith("/users/"):
            identifier = parsed.path[len("/users/"):]
            if "page" in parsed.query:
                value = parsed.query.split("page=", 1)[1].split("&", 1)[0]
                if not value.lstrip("-").isdigit() or int(value) < 0:
                    reply(self, 400, {"error": "bad page"})
                    return
            if len(identifier) > 32:
                reply(self, 500, {"error": "id too long"})
                return
            if identifier == "sample":
                reply(self, 200, {"id": identifier, "name": "Sample"})
                return
            reply(self, 404, {"error": "not found"})
            return
        reply(self, 404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/users":
            reply(self, 404, {"error": "not found"})
            return
        length = int(self.headers.get("Content-Length", "0"))
        try:
            body = json.loads(self.rfile.read(length) or b"{}")
        except ValueError:
            reply(self, 400, {"error": "invalid json"})
            return
        name = body.get("name")
        role = body.get("role")
        if not isinstance(name, str) or not name:
            reply(self, 500, {"error": "name validation crashed"})
            return
        if role is not None and role not in ("admin", "user"):
            reply(self, 400, {"error": "bad role"})
            return
        reply(self, 201, {"id": "new", "name": name, "role": role})

    def log_message(self, *_args):
        pass


if __name__ == "__main__":
    HTTPServer(("0.0.0.0", PORT), Handler).serve_forever()
