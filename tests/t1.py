import asyncio

from asyncio import AbstractEventLoop
from typing import Optional, Callable

import pyre


loop = asyncio.SelectorEventLoop()
asyncio.set_event_loop(loop)


async def callback(
    send: pyre.DataSender,
    *args,
):
    send(
        False,
        b"HTTP/1.1 200 OK\r\n"
        b"Content-Length: 13\r\n"
        b"Server: Pyre\r\n"
        b"\r\n"
        b"Hello, World!"
    )


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


class PartialTask:
    """
    A partial task factory, when called it produces a task of the
    callback with the give args and kwargs.
    """
    def __init__(self, loop_: asyncio.AbstractEventLoop, cb):
        self.loop = loop_
        self.cb = cb

    def __call__(self, *args, **kwargs):
        self.loop.create_task(self.cb(*args, **kwargs))


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
            idle_max: int = -1,
            loop: Optional[AbstractEventLoop] = None,
    ):
        self.host = host
        self.port = port
        self.debug = debug
        self.backlog = backlog
        self.keep_alive = keep_alive
        self.idle_max = idle_max
        self.loop = loop or asyncio.get_event_loop()

        self._factory = PartialTask(self.loop, app)

        self._server: pyre.Server = pyre.create_server(
            self.host,
            self.port,
            self._factory,
            self.backlog,
            self.keep_alive,
            self.idle_max if idle_max > 0 else 0,
        )

        self._server.init(
            self._add_reader,
            self._remove_reader,
            self._add_writer,
            self._remove_writer,
        )

        self._waiter = asyncio.Future()

    def shutdown(self):
        self._server.shutdown()
        self._waiter.set_result(None)

    def start(self):
        self._server.start(self.loop.add_reader, self._server.poll_accept)
        self.loop.create_task(self.keep_alive_ticker())

        if self.idle_max > 0:
            self.loop.create_task(self.idle_max_ticker())
        self._server.poll_accept()

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

    async def idle_max_ticker(self):
        while not self._waiter.done():
            try:
                self._server.poll_idle()
            except Exception as e:
                print("Unhandled keep alive exception: {}".format(e))
            await asyncio.sleep(self.idle_max)

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


async def main():
    print("Running @ http://127.0.0.1:8080")
    server = Server(callback, host="0.0.0.0", port=8080)
    server.start()
    try:
        await server.run_forever()
    except KeyboardInterrupt:
        print("Shutting down server")
        server.shutdown()

loop.run_until_complete(main())
