import asyncio

from .shared import Executor, Waiter


class AsyncioWaiter(Waiter):
    def __init__(self, fut: asyncio.Future):
        self._fut = fut

    async def wait(self):
        """
        Waits via pending until the waiter is stopped using `Waiter.stop()`
        """
        await self._fut

    def is_done(self) -> bool:
        """ A check to determine if the waiter is done or not. """
        return self._fut.done()

    def stop(self):
        """
        Stop the waiter from pending releasing any `await Waiter.wait()`'s
        """
        self._fut.set_result(None)


class AsyncioExecutor(Executor):
    def __init__(self, loop: asyncio.AbstractEventLoop = None):
        self._loop = loop or asyncio.get_running_loop()

    def create_task(self, cb, *args):
        """ Spawns a concurrent task in a non-blocking manor """
        self._loop.create_task(cb(*args))

    def add_writer(self, fd, callback, *args):
        """
        Add the callback with a given file descriptor to something and
        call the callback when the file descriptor is ready to be written to
        """
        self._loop.add_writer(fd, callback, *args)

    def add_reader(self, fd, callback, *args):
        """
        Add the callback with a given file descriptor to something and
        call the callback when the file descriptor is ready to be read from
        """
        self._loop.add_reader(fd, callback, *args)

    def remove_writer(self, fd):
        """
        Remove the file descriptor from the handler that invokes the writer
        callback.
        """
        self._loop.remove_writer(fd)

    def remove_reader(self, fd):
        """
        Remove the file descriptor from the handler that invokes the reader
        callback.
        """
        self._loop.remove_reader(fd)

    def create_waiter(self) -> Waiter:
        """
        Produce a waiter class or child that can be used internally.
        """
        fut = self._loop.create_future()
        return AsyncioWaiter(fut)

    async def sleep(self, n: float):
        """
        Suspend the current task in a non-blocking fashion for n time.
        """
        await asyncio.sleep(n)

