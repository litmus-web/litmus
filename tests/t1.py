import asyncio
# import uvloop
from pyre.server import Server

# uvloop.install()
loop = asyncio.SelectorEventLoop()
asyncio.set_event_loop(loop)


def suprise(send, *args):
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

    server = Server(suprise, host="0.0.0.0", port=8080)
    server.start()
    try:
        await server.run_forever()
    except KeyboardInterrupt:
        print("Shutting down server")
        server.shutdown()

loop.run_until_complete(main())
