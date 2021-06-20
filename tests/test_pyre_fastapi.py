import asyncio


from litmus.server.adapters import PSGIToASGIAdapter
from litmus.server import Server

try:
    import uvloop
    uvloop.install()
except ImportError:
    loop = asyncio.SelectorEventLoop()
    asyncio.set_event_loop(loop)


async def suprise(
    scope,
    receive,
    send,
):
    await send({
        "type": "http.response.start",
        "status": 200,
        "headers": (
            (b"Content-Length", b"13"),
            (b"Content-Type", b"text/plain"),
        ),
    })

    await send({
        "type": "http.response.body",
        "more_body": False,
        "body": b"Hello, World!"
    })


async def main():
    wrapped = PSGIToASGIAdapter(suprise)
    server = Server(wrapped, host="0.0.0.0", port=8080)
    server.start()
    try:
        await server.run_forever()
    except KeyboardInterrupt:
        print("Shutting down server")
        server.shutdown()

asyncio.get_event_loop().run_until_complete(main())