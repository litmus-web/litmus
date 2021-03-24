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
    try:
        body = receive()
        print(body)
    except BlockingIOError:
        fut = asyncio.get_event_loop().create_future()
        receive.subscribe(fut.set_result)
        print("waiting")
        print(await fut)

    send(
        # more body
        False,
        # body
        b"HTTP/1.1 200 OK\r\n"
        b"Content-Length: 13\r\n"
        b"Content-Type: text/plain\r\n"
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

asyncio.get_event_loop().run_until_complete(main())
