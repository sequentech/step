# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import asyncio
import json
import unittest

from .http_pool import HttpError, HttpPool, parse_head


class ParseHeadTest(unittest.TestCase):
    def test_status_and_lower_cased_headers(self):
        status, headers = parse_head(
            b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nX-Thing: a: b\r\n\r\n"
        )
        self.assertEqual(status, 200)
        self.assertEqual(headers, {"content-length": "12", "x-thing": "a: b"})

    def test_a_line_that_is_not_http_is_refused(self):
        with self.assertRaises(HttpError):
            parse_head(b"SSH-2.0-OpenSSH\r\n\r\n")


class Server:
    """A local HTTP/1.1 server answering each POST with its JSON body echoed."""

    def __init__(self, chunked=False, close_after=None):
        self.chunked = chunked
        self.close_after = close_after
        self.connections = 0
        self.requests = 0
        self.open = 0
        self.peak = 0

    async def handle(self, reader, writer):
        self.connections += 1
        self.open += 1
        self.peak = max(self.peak, self.open)
        served = 0
        try:
            while True:
                head = await reader.readuntil(b"\r\n\r\n")
                length = 0
                for line in head.decode().split("\r\n")[1:]:
                    name, _, value = line.partition(":")
                    if name.lower() == "content-length":
                        length = int(value)
                body = await reader.readexactly(length)
                self.requests += 1
                served += 1
                await asyncio.sleep(0.01)
                answer = json.dumps({"echo": json.loads(body)}).encode()
                close = self.close_after is not None and served >= self.close_after
                extra = "Connection: close\r\n" if close else ""
                if self.chunked:
                    half = len(answer) // 2
                    payload = (
                        f"{half:x}\r\n".encode()
                        + answer[:half]
                        + b"\r\n"
                        + f"{len(answer) - half:x}\r\n".encode()
                        + answer[half:]
                        + b"\r\n0\r\n\r\n"
                    )
                    writer.write(
                        (
                            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n"
                            f"{extra}\r\n"
                        ).encode()
                        + payload
                    )
                else:
                    writer.write(
                        (
                            f"HTTP/1.1 200 OK\r\nContent-Length: {len(answer)}\r\n"
                            f"{extra}\r\n"
                        ).encode()
                        + answer
                    )
                await writer.drain()
                if close:
                    break
        except (asyncio.IncompleteReadError, ConnectionResetError):
            pass
        finally:
            self.open -= 1
            writer.close()


class HttpPoolTest(unittest.IsolatedAsyncioTestCase):
    async def serve(self, server):
        listener = await asyncio.start_server(server.handle, "127.0.0.1", 0)
        self.addAsyncCleanup(self.stop, listener)
        port = listener.sockets[0].getsockname()[1]
        return f"http://127.0.0.1:{port}/v1/graphql"

    async def stop(self, listener):
        listener.close()
        await listener.wait_closed()

    async def test_keeps_connections_alive_and_within_the_limit(self):
        server = Server()
        pool = HttpPool(await self.serve(server), limit=3)
        answers = await asyncio.gather(
            *(pool.post_json({"n": index}) for index in range(20))
        )
        await pool.close()
        self.assertEqual([body["echo"]["n"] for _, body in answers], list(range(20)))
        self.assertEqual({status for status, _ in answers}, {200})
        self.assertEqual(server.requests, 20)
        self.assertLessEqual(server.peak, 3)
        self.assertLessEqual(server.connections, 3)

    async def test_reads_chunked_answers(self):
        pool = HttpPool(await self.serve(Server(chunked=True)), limit=1)
        status, body = await pool.post_json({"query": "x" * 100})
        await pool.close()
        self.assertEqual((status, body), (200, {"echo": {"query": "x" * 100}}))

    async def test_reconnects_after_the_server_closes(self):
        server = Server(close_after=1)
        pool = HttpPool(await self.serve(server), limit=1)
        for index in range(3):
            _, body = await pool.post_json({"n": index})
            self.assertEqual(body, {"echo": {"n": index}})
        await pool.close()
        self.assertEqual(server.connections, 3)

    async def test_sends_the_given_headers(self):
        seen = []

        async def handle(reader, writer):
            head = await reader.readuntil(b"\r\n\r\n")
            seen.append(head.decode())
            length = int(head.decode().lower().split("content-length:")[1].split()[0])
            await reader.readexactly(length)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            await writer.drain()
            writer.close()

        listener = await asyncio.start_server(handle, "127.0.0.1", 0)
        self.addAsyncCleanup(self.stop, listener)
        port = listener.sockets[0].getsockname()[1]
        pool = HttpPool(f"http://127.0.0.1:{port}/v1/graphql", limit=1)
        await pool.post_json({}, headers={"Authorization": "Bearer t"})
        await pool.close()
        self.assertIn("POST /v1/graphql HTTP/1.1", seen[0])
        self.assertIn("Authorization: Bearer t", seen[0])
        self.assertIn(f"Host: 127.0.0.1:{port}", seen[0])


if __name__ == "__main__":
    unittest.main()
