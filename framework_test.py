import asyncio

from pyre.framework import Blueprint, endpoint, router


class Test(Blueprint):
    def __init__(self):
        ...

    @endpoint("/hello/{foo:string}")
    async def foo(self, req):
        print("wew")

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
    t = Test()

    router.apply_methods(t)

    print(t.foo.route)
    print(t._endpoints)
    asyncio.run(t.foo(""))
