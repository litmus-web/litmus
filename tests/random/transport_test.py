import asyncio
import pyre
import time


count = 0
last_set = 0


import logging

logging.basicConfig(level=logging.DEBUG)


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
        False,
    )
    asyncio.get_event_loop().call_later(5, send.close)
    send.flush()


def factory():
    global count, last_set

    count += 1

    # print(f"req no: {count}, time: {time.time() - last_set}")
    last_set = time.time()

    return pyre.RustProtocol(tet)


host = "0.0.0.0"
port = 80


async def main():
    loop = asyncio.get_event_loop()
    server = await loop.create_server(factory, host=host, port=port, backlog=1024)
    print(f"Running on: http://{host}:{port}")
    await server.serve_forever()

asyncio.run(main())