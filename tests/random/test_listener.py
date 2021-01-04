import asyncio
import selectors
import functools

import pyre

selector = selectors.SelectSelector()
loop = asyncio.SelectorEventLoop(selector)
asyncio.set_event_loop(loop)


async def test(s, r):
    msg = "Hello, World!"
    await s(b"HTTP/1.1 200 OK\r\n"
            b"Content-Length: %d\r\n"
            b"Content-Type: text/plain; charset=UTF-8\r\n\r\n"
            b"%s" % (len(msg), msg.encode("utf-8")))


class Server:
    def __init__(self, loop_):
        self._loop: asyncio.AbstractEventLoop = loop_
        pyre.setup(
            self.task_factory,
            self._loop.remove_reader,
            self._loop.remove_writer,
        )
        self._listener = pyre.PyreListener(test, host="0.0.0.0", port=8080)

        self.backlog = 1024
        self.fd = 0

    def start_server(self):
        self.fd = self._listener.fd()
        fut = self._loop.create_future()

        self._loop.add_reader(self.fd, self.accept_connections, fut)

        return fut

    def accept_connections(self, _future):
        print("got conn")
        for _ in range(self.backlog):
            try:
                handle = self._listener.accept()
            except BlockingIOError:
                return
            else:
                fd = handle.fd()
                handle.init(
                    functools.partial(self._loop.add_reader, callback=handle.poll_read),
                    functools.partial(self._loop.add_writer, callback=handle.poll_write),
                )
                self._loop.add_reader(fd, handle.poll_read)

    def task_factory(self, *args):
        self._loop.create_task(test(*args))


async def main():
    server = Server(loop)
    await server.start_server()
    print("wew")


loop.run_until_complete(main())
