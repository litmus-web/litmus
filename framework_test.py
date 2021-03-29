import asyncio

from pyre.server import Server
from pyre.framework import Blueprint, endpoint, App, responses

try:
    import uvloop
    uvloop.install()
except ImportError:
    loop = asyncio.SelectorEventLoop()
    asyncio.set_event_loop(loop)


app = App()


class Test(Blueprint):
    def __init__(self):
        ...

    @endpoint("/hello")
    async def t6(self, _):
        return responses.TextResponse("hello")


if __name__ == '__main__':
    app.add_blueprint(Test())


    async def main():
        print("Running @ http://127.0.0.1:8080")

        server = Server(app.psgi_app, host="0.0.0.0", port=8080)
        server.start()
        try:
            await server.run_forever()
        except KeyboardInterrupt:
            print("Shutting down server")
            server.shutdown()


    asyncio.get_event_loop().run_until_complete(main())

