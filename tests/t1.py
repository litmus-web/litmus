import asyncio
# import uvloop
from pyre.server import Server

# uvloop.install()
loop = asyncio.SelectorEventLoop()
asyncio.set_event_loop(loop)


async def suprise(scope, send, receive):
    print(scope, send, receive)


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
