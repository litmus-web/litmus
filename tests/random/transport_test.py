import asyncio
import pyre


async def tet(*args):
    await asyncio.sleep(10)


async def main():
    loop = asyncio.get_event_loop()
    server = await loop.create_server(lambda: pyre.RustProtocol(tet), host="127.0.0.1", port=6060)
    await server.serve_forever()

asyncio.run(main())