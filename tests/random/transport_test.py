import asyncio
import pyre


loop = asyncio.get_event_loop()


def factory() -> pyre.PyreProtocol:
    return pyre.PyreProtocol()


host = "0.0.0.0"
port = 80


async def main():
    loop = asyncio.get_event_loop()
    server = await loop.create_server(factory, host=host, port=port, backlog=1024)
    print(f"Running on: http://{host}:{port}")
    await server.serve_forever()

asyncio.run(main())