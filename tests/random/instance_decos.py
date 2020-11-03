import typing as t


class Endpoint:
    def __init__(
            self,
            route: str,
            callback: t.Callable,
            before_invoke: t.Optional[t.Callable],
            on_error: t.Optional[t.Callable],
            **kwargs,
    ):
        self.route = route
        self.callback = callback
        self.before_invoke = before_invoke
        self.on_error = on_error
        self.kwargs = kwargs

    async def __call__(self, request, *args):
        try:
            if self.before_invoke is not None:
                await self.before_invoke(request, *args)
            return await self.callback(request, *args)
        except Exception as e:
            if self.on_error is not None:
                await self.on_error(request, e)
                return e
            raise e


class Wrapper:
    __error_name = ""
    __middle_name = ""

    def __init__(self, route, callback, **kwargs):
        self.route = route
        self.kwargs = kwargs
        self.callback_name = callback.__name__
        self.original_callback = callback

    def to_endpoint(self, instance):
        callback = getattr(instance, self.callback_name)
        error_handler = getattr(instance, self.error_handler_name, None)
        before_invoke = getattr(instance, self.middle_handler_name, None)

        return Endpoint(
            route=self.route,
            callback=callback,
            before_invoke=before_invoke,
            on_error=error_handler,
            **self.kwargs
        )

    @property
    def error_handler_name(self):
        return self.__error_name

    @property
    def middle_handler_name(self):
        return self.__middle_name

    @classmethod
    def error(cls, func):
        cls.__error_name = func.__name__
        return func

    @classmethod
    def before_invoke(cls, func):
        cls.__middle_name = func.__name__
        return func


def endpoint(route: str, **kwargs):
    def wrapper(func):
        return Wrapper(route, func, **kwargs)
    return wrapper


class Blueprint:
    _endpoints = []

    def __init_subclass__(cls, **kwargs):
        cls._endpoints = []

        for k, v in cls.__dict__.items():
            if k.startswith("__") or k.endswith("__"):
                continue

            if isinstance(v, Wrapper):
                cls._endpoints.append(v)
                setattr(cls, v.callback_name, v.original_callback)

    async def invoke_endpoint(self, ep, request):
        try:
            return await ep(request, *request.args)
        except Exception as e:
            await self.on_blueprint_error(request, e)

    async def on_blueprint_error(self, request, exception):
        raise exception


class MyTestClass(Blueprint):
    def __init__(self):
        self.x = "abc"

    async def on_blueprint_error(self, request, exception: Exception):
        raise exception

    @endpoint("/hello/{x:string}/{id:uuid}")
    async def my_endpoint(self):
        print(self.x)
        raise Exception("ahhh")

    @my_endpoint.before_invoke
    async def my_endpoint_middle_wear(self, *args):
        print("first")

    @my_endpoint.error
    async def my_endpoint_error(self, *args):
        print("handling exception")


def apply_methods(instance):
    endpoints: t.List[Wrapper] = instance._endpoints
    for ep in endpoints:
        setattr(instance, ep.callback_name, ep.to_endpoint(instance))


if __name__ == '__main__':
    test = MyTestClass()
    print(type(test.my_endpoint))
    print(type(test.my_endpoint_error))
    print(type(test.my_endpoint_middle_wear))

    apply_methods(test)

    import asyncio

    asyncio.run(test())



