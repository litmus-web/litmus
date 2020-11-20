import asyncio
import logging
import typing as t

logging.basicConfig(level=logging.DEBUG)


class HTTPProtocol(asyncio.Protocol):
    def __init__(self):
        self.transport: t.Optional[asyncio.Transport] = None

    def connection_made(self, transport: asyncio.Protocol) -> None:
        self.transport = transport

    def data_received(self, data: bytes) -> None:
        self.transport.write(
            b"HTTP/1.1 200 OK\r\n"
            b"content-type: text/plain\r\n"
            b"transfer-encoding: chunked\r\n"
            b"\r\n"
        )

        self.transport.write(
            b"d\r\nHello, world!\r\n0\r\n\r\n"
        )


async def main(host, port):
    loop = asyncio.get_running_loop()
    server = await loop.create_server(HTTPProtocol, host, port)
    await server.serve_forever()


asyncio.run(main('0.0.0.0', 5000))
