from typing import Callable

from .. import _Server, create_server


class Waiter:
    """
    Implements the methods Pyre expects to exist in order to handle
    the server interactions.
    """

    async def wait(self):
        """
        Waits via pending until the waiter is stopped using `Waiter.stop()`
        """
        raise NotImplemented()

    def is_done(self) -> bool:
        """ A check to determine if the waiter is done or not. """
        raise NotImplemented()

    def stop(self):
        """
        Stop the waiter from pending releasing any `await Waiter.wait()`'s
        """
        raise NotImplemented()


class Executor:
    """
    Implements the methods Pyre expects to exist in order to handle
    the server interactions.
    """

    def create_task(self, cb, *args):
        """ Spawns a concurrent task in a non-blocking manor """
        raise NotImplemented()

    def add_writer(self, fd, callback, *args):
        """
        Add the callback with a given file descriptor to something and
        call the callback when the file descriptor is ready to be written to
        """
        raise NotImplemented()

    def add_reader(self, fd, callback, *args):
        """
        Add the callback with a given file descriptor to something and
        call the callback when the file descriptor is ready to be read from
        """
        raise NotImplemented()

    def remove_writer(self, fd):
        """
        Remove the file descriptor from the handler that invokes the writer
        callback.
        """
        raise NotImplemented()

    def remove_reader(self, fd):
        """
        Remove the file descriptor from the handler that invokes the reader
        callback.
        """
        raise NotImplemented()

    def create_waiter(self) -> Waiter:
        """
        Produce a waiter class or child that can be used internally.
        """
        raise NotImplemented()

    async def sleep(self, n: float):
        """
        Suspend the current task in a non-blocking fashion for n time.
        """
        raise NotImplemented()


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

    def __init__(self, executor: Executor, cb):
        self.executor = executor
        self.cb = cb

    def __call__(self, *args, **kwargs):
        self.executor.create_task(self.cb, *args, **kwargs)


class Server:
    def __init__(
            self,
            app: Callable,
            executor: Executor,
            host: str = "127.0.0.1",
            port: int = 8080,
            *,
            debug: bool = False,
            backlog: int = 1024,
            keep_alive: int = 5,
            idle_max: int = -1,
    ):
        self.host = host
        self.port = port
        self.debug = debug
        self.backlog = backlog
        self.keep_alive = keep_alive
        self.idle_max = idle_max
        self.executor = executor

        self._waiter = self.executor.create_waiter()
        self._factory = PartialTask(self.executor, app)

        self._server: _Server = create_server(
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

    def shutdown(self):
        self._server.shutdown()
        self._waiter.stop()

    def start(self):
        self._server.start(self.executor.add_reader, self._server.poll_accept)
        self.executor.create_task(self.keep_alive_ticker)

        if self.idle_max > 0:
            self.executor.create_task(self.idle_max_ticker)
        self._server.poll_accept()

    async def run_forever(self):
        await self._waiter.wait()

    async def keep_alive_ticker(self):
        while not self._waiter.is_done():
            if self.debug:
                print("Active Clients: ", self._server.len_clients())
            try:
                self._server.poll_keep_alive()
            except Exception as e:
                print("Unhandled keep alive exception: {}".format(e))
            await self.executor.sleep(self.keep_alive)

    async def idle_max_ticker(self):
        while not self._waiter.is_done():
            try:
                self._server.poll_idle()
            except Exception as e:
                print("Unhandled keep alive exception: {}".format(e))
            await self.executor.sleep(self.idle_max)

    @property
    def _add_reader(self):
        return FileDescriptorPartial(
            self.executor.add_reader,
            callback=self._server.poll_read
        )

    @property
    def _remove_reader(self):
        return self.executor.remove_reader

    @property
    def _add_writer(self):
        return FileDescriptorPartial(
            self.executor.add_writer,
            callback=self._server.poll_write
        )

    @property
    def _remove_writer(self):
        return self.executor.remove_writer
