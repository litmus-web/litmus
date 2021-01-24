import trio

import pyre
from pyre.server import Server, TrioExecutor


async def surpise(
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


async def main():
    print("Running @ http://127.0.0.1:8080")

    async with trio.open_nursery() as nursery:
        executor = TrioExecutor(nursery)
        server = Server(surpise, executor=executor, host="0.0.0.0", port=8080)
        server.start()
        try:
            await server.run_forever()
        except KeyboardInterrupt:
            print("Shutting down server")
            server.shutdown()

trio.run(main)
