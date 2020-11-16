import asyncio
import pyre


loop = asyncio.get_event_loop()


async def tet(send, recv):
    await send(
        200,
        [
            (b"content-type", b"text/plain"),
            (b"transfer-encoding", b"chunked"),
        ],
        b"",
        True,
    )

    await send(
        0,
        [],
        b"d\r\nHello, World!\r\n0\r\n\r\n",
        True,
    )
    asyncio.get_event_loop().call_later(5, send.close)
    send.flush()


import itertools

counter = itertools.count(1, 1)


def factory() -> pyre.PyreProtocol:
    # print("req", next(counter), sep=" ")
    return pyre.PyreProtocol(tet)


host = "0.0.0.0"
port = 80





async def main():
    loop = asyncio.get_event_loop()
    server = await loop.create_server(factory, host=host, port=port, backlog=1024)
    print(f"Running on: http://{host}:{port}")
    await server.serve_forever()

asyncio.run(main())