import asyncio
import pyre


async def tet(send, recv):
    await send(
        200,
        [
            (b"content-type", b"text/html; charset=UTF-8"),
            (b"content-length", b"0"),
            (b"connection", b"close"),
        ],
        b"",
        False,
    )

    await asyncio.sleep(0.5)


host = "0.0.0.0"
port = 80


async def main():
    loop = asyncio.get_event_loop()
    server = await loop.create_server(lambda: pyre.RustProtocol(tet), host=host, port=port)
    print(f"Running on: http://{host}:{port}")
    await server.serve_forever()

asyncio.run(main())