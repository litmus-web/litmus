import uvicorn

from pyre import framework
from pyre.framework import Blueprint, endpoint, App, responses

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

    uvicorn.run(app, host="127.0.0.1", port=8080)
