import trio

from .shared import Executor, Waiter


class TrioWaiter(Waiter):
    def __init__(self):
        self._event = trio.Event()

    async def wait(self):
        """
        Waits via pending until the waiter is stopped using `Waiter.stop()`
        """
        await self._event.wait()

    def is_done(self) -> bool:
        """ A check to determine if the waiter is done or not. """
        return self._event.is_set()

    def stop(self):
        """
        Stop the waiter from pending releasing any `await Waiter.wait()`'s
        """
        self._event.set()


class TrioSocketHandler:
    def __init__(self):
        self._continue = True

    def cancel(self):
        self._continue = False

    async def wait_for_write(self, fd, callback, args):
        while self._continue:
            await trio.lowlevel.wait_writable(fd)
            callback(*args)

    async def wait_for_read(self, fd, callback, args):
        while self._continue:
            await trio.lowlevel.wait_readable(fd)
            callback(*args)


class TrioExecutor(Executor):
    def __init__(self, nursery: trio.Nursery):
        self._nursery = nursery
        self._write_handlers = {}
        self._read_handlers = {}

    def create_task(self, cb, *args):
        """ Spawns a concurrent task in a non-blocking manor """
        self._nursery.start_soon(cb, *args)

    def add_writer(self, fd, callback, *args):
        """
        Add the callback with a given file descriptor to something and
        call the callback when the file descriptor is ready to be written to
        """
        if fd in self._write_handlers:
            raise ValueError(
                "FileDescriptor is already being watched for writing")

        handler = TrioSocketHandler()
        self._nursery.start_soon(handler.wait_for_write, fd, callback, args)
        self._write_handlers[fd] = handler

    def add_reader(self, fd, callback, *args):
        """
        Add the callback with a given file descriptor to something and
        call the callback when the file descriptor is ready to be read from
        """
        if fd in self._read_handlers:
            raise ValueError(
                "FileDescriptor is already being watched for reading")

        handler = TrioSocketHandler()
        self._nursery.start_soon(handler.wait_for_read, fd, callback, args)
        self._read_handlers[fd] = handler

    def remove_writer(self, fd):
        """
        Remove the file descriptor from the handler that invokes the writer
        callback.
        """

        handler = self._write_handlers.pop(fd, None)
        if handler is None:
            raise ValueError(
                "FileDescriptor is not being watched for writing")

        handler.cancel()

    def remove_reader(self, fd):
        """
        Remove the file descriptor from the handler that invokes the reader
        callback.
        """
        handler = self._read_handlers.pop(fd, None)
        if handler is None:
            raise ValueError(
                "FileDescriptor is not being watched for reading")

        handler.cancel()

    def create_waiter(self) -> Waiter:
        """
        Produce a waiter class or child that can be used internally.
        """
        return TrioWaiter()

    async def sleep(self, n: float):
        """
        Suspend the current task in a non-blocking fashion for n time.
        """
        await trio.sleep(n)

