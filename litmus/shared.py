import asyncio
from typing import List
from functools import partial

from . import _Server, create_server


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
        app_callback,
        listen_on: List[str] = "127.0.0.1:8080",
        backlog: int = 1024,
        keep_alive: int = 5,
        gc_interval: int = 60,
        keep_alive_interval: int = 1,
    ):
        if isinstance(listen_on, str):
            listen_on = [listen_on]

        self.app = app_callback
        self.loop = asyncio.get_running_loop()
        self.gc_interval = gc_interval
        self.keep_alive_interval = keep_alive_interval

        if hasattr(asyncio, "ProactorEventLoop") and isinstance(self.loop, asyncio.ProactorEventLoop):
            raise TypeError("the asyncio.ProactorEventLoop event loop is not supported")

        self._waiter = self.loop.create_future()
        self._shutdown = False

        self._server = create_server(
            self.__app,
            listen_on,
            backlog,
            keep_alive,
        )
        self._server.init(
            self._add_reader,
            self._remove_reader,
            self._add_writer,
            self._remove_writer,
            self._close_socket,
        )
        self._kai_task = self.loop.call_later(self.keep_alive_interval, self._poll_keep_alive)

    def _poll_keep_alive(self):
        self._server.poll_keep_alive()

        if not self._shutdown:
            self.loop.call_later(
                self.keep_alive_interval,
                self._poll_keep_alive,
            )

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

    def _register_listener(self, fd: int, index: int):
        self.loop.add_reader(fd, self._server.poll_accept, index)

    def ignite(self):
        self._server.ignite(self._register_listener)

    def shutdown(self):
        self._server.shutdown()
        self._shutdown = True
        self._gci_task.cancel()
        self._kai_task.cancel()
        self._waiter.set_result(None)

    async def run_forever(self):
        await self._waiter
