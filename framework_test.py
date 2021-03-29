import asyncio

from pyre.server import Server
from pyre.framework import Blueprint, endpoint, App, responses

try:
    import uvloop
    uvloop.install()
except ImportError:
    loop = asyncio.SelectorEventLoop()
    asyncio.set_event_loop(loop)


"""async def suprise(
    scope,
    send,
    receive,
):
    # print(pprint(scope))
    try:
        body = receive()
        # print(body)
    except BlockingIOError:
        fut = asyncio.get_event_loop().create_future()
        receive.subscribe(fut.set_result)
        # print("waiting")
        # print(await fut)

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
    )"""


app = App()


class Test(Blueprint):
    def __init__(self):
        ...

    @endpoint("/hello/{foo:string}")
    async def foo(self, _, foo):
        return responses.TextResponse(f"hello, {foo}!")

    @foo.error
    async def foo_error(self, req, err):
        ...

    @foo.before_invoke
    async def foo_middleware(self, req):
        ...

    @endpoint("/hello/{foo:string}")
    async def bar(self, req):
        print("wew")


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
