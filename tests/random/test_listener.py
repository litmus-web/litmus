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
            test,
            self.task_factory,
            self._loop.remove_reader,
            self._loop.remove_writer,
        )
        self._listener = pyre.PyreListener("0.0.0.0:8080")

        self.backlog = 1024
        self.fd = 0

    def start_server(self):
        self.fd = self._listener.fileno()
        fut = self._loop.create_future()

        self._loop.add_reader(self.fd, self.accept_connections, fut)

        return fut

    def accept_connections(self, future):
        for _ in range(self.backlog):
            try:
                client = self._listener.accept()
            except BlockingIOError:
                return
            else:
                fd = client.fd()
                client.init(
                    functools.partial(
                        self._loop.add_reader, fd, client, "POLL_READ"),
                    functools.partial(
                        self._loop.add_writer, fd, client, "POLL_WRITE"),
                )
                self._loop.add_reader(fd, client, "POLL_READ")

    def task_factory(self, *args):
        self._loop.create_task(test(*args))


async def main():
    server = Server(loop)
    await server.start_server()
    print("wew")


loop.run_until_complete(main())
