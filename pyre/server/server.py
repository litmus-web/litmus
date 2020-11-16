import asyncio
import logging
import functools
import typing as t

from ssl import SSLContext

from ..pyre import PyreProtocol


class PyreServer:
    def __init__(
            self,
            app: t.Callable,
            host: str = "127.0.0.1",
            port: int = 5050,
            reuse_port: t.Optional[bool] = None,
            reuse_addr: t.Optional[bool] = None,
            ssl: t.Optional[SSLContext] = None,
            ssl_handshake_timeout: t.Optional[float] = None,
            backlog: int = 100,
            loop: t.Optional[asyncio.AbstractEventLoop] = None,
            **options
    ):
        if loop is None:
            loop = asyncio.get_event_loop()

        self.app = app
        self.host = host
        self.port = port
        self.reuse_port = reuse_port
        self.reuse_addr = reuse_addr
        self.ssl = ssl
        self.ssl_handshake_timeout = ssl_handshake_timeout
        self.backlog = backlog
        self.loop = loop
        self.server_options = options

        self.server: t.Optional[asyncio.AbstractServer] = None

    def ignite(self):
        try:
            self.loop.run_until_complete(self._run())
        except Exception as e:
            if self.server is not None:
                self.server.close()
            raise e

    async def _run(self):
        cb = functools.partial(
            PyreProtocol,
            self.app,
        )

        server = await self.loop.create_server(
            protocol_factory=cb,
            host=self.host,
            port=self.port,
            reuse_port=self.reuse_port,
            reuse_address=self.reuse_addr,
            ssl=self.ssl,
            ssl_handshake_timeout=self.ssl_handshake_timeout,
            backlog=self.backlog,
            **self.server_options
        )