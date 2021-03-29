import asyncio

from pyre.server import Server

try:
    import uvloop
    uvloop.install()
except ImportError:
    loop = asyncio.SelectorEventLoop()
    asyncio.set_event_loop(loop)


async def suprise(
    scope,
    send,
    receive,
):
    send.send_start(
        200,
        (
            (b"Content-Length", b"13"),
            (b"Content-Type", b"text/plain"),
        )
    )

    send.send_body(
        False,
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

asyncio.get_event_loop().run_until_complete(main())
