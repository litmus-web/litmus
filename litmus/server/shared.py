import asyncio
from typing import Callable
from functools import partial

from .. import _Server, create_server


class FileDescriptorPartial:
    """
    Creates a partial callback with a preset callback set,
    this means that only a file descriptor and index is needed in oder
    to add a fd listener to the event loop, this is mostly just utils
    for the internal Rust handlers to ensure the callbacks are available
    from the heap.

    Args:
        caller:
            The callable that will be invoked when an instance of this
            class is called.
        callback:
            The callable that is passed to the caller as a preset callback
            kwarg.

    Returns:
        A instance of `FileDescriptorPartial` read to be called.
    """

    __slots__ = ('_caller', '_callback')

    def __init__(self, caller, callback):
        self._caller = caller
        self._callback = callback

    def __call__(self, fd: int, index: int, *args):
        self._caller(fd, self._callback, index)


class Server:
    def __init__(
            self,
            app: Callable,
            host: str = "127.0.0.1",
            port: int = 8080,
            *,
            debug: bool = False,
            backlog: int = 1024,
            keep_alive: int = 5,
            loop: asyncio.AbstractEventLoop = None
    ):
        self.app = app
        self.host = host
        self.port = port
        self.debug = debug
        self.backlog = backlog
        self.keep_alive = keep_alive
        self.loop = loop or asyncio.get_event_loop()

        self._waiter = self.loop.create_future()

        self._server: _Server = create_server(
            self.host,
            self.port,
            self.__app,
            self.backlog,
            self.keep_alive,
        )

        self._server.init(
            self._add_reader,
            self._remove_reader,
            self._add_writer,
            self._remove_writer,
            self._close_socket,
        )

    def shutdown(self):
        self._server.shutdown()
        self._waiter.set_result(None)

    def start(self):
        self._server.start(self.loop.add_reader, self._server.poll_accept)
        self.loop.create_task(self.keep_alive_ticker())
        self._server.poll_accept()

    def __app(self, scope, send, receive):
        scope = {
            "type": scope[0],
            "http_version": scope[1],
            "method": scope[2],
            "scheme": scope[3],
            "path": scope[4],
            "query": scope[5],
            "root_path": scope[6],
            "headers": scope[7],
            "client": scope[8],
            "server": scope[9],
        }

        self.loop.create_task(self.app(scope, send, receive))

    async def run_forever(self):
        await self._waiter

    async def keep_alive_ticker(self):
        while not self._waiter.done():
            if self.debug:
                print("Active Clients: ", self._server.len_clients())
            try:
                self._server.poll_keep_alive()
            except Exception as e:
                print("Unhandled keep alive exception: {}".format(e))
            await asyncio.sleep(self.keep_alive)

    @property
    def _add_reader(self):
        return FileDescriptorPartial(
            self.loop.add_reader,
            callback=self._server.poll_read
        )

    @property
    def _remove_reader(self):
        return self.loop.remove_reader

    @property
    def _add_writer(self):
        return FileDescriptorPartial(
            self.loop.add_writer,
            callback=self._server.poll_write
        )

    @property
    def _remove_writer(self):
        return self.loop.remove_writer

    @property
    def _close_socket(self):
        return partial(self.loop.call_soon, self._server.poll_close)