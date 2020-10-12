import typing as t
import inspect
import sys


class Endpoint:
    def __init__(self, callback: t.Callable, route: str, methods: t.List[str]):
        self.callback: t.Optional[t.Callable] = None
        self._callback_name = callback.__name__
        self._route = route
        self._methods = methods

    def __repr__(self):
        return "Endpoint(route={})".format(repr(self._route))

    def __call__(self, *args, **kwargs):
        return self.callback(*args, **kwargs)

    @property
    def __name__(self):
        return self._callback_name

    @property
    def route(self):
        return self._route

    @property
    def methods(self):
        return self._methods

    @property
    def callback_name(self):
        return self._callback_name


class Websocket:
    def __init__(self, callback: t.Callable, route: str):
        self.callback: t.Optional[t.Callable] = None
        self._callback_name = callback.__name__
        self._route = route

    def __repr__(self):
        return "Websocket(route={})".format(repr(self._route))

    def __call__(self, *args, **kwargs):
        return self.callback(*args, **kwargs)

    @property
    def __name__(self):
        return self._callback_name

    @property
    def route(self):
        return self._route

    @property
    def callback_name(self):
        return self._callback_name


pending_endpoints = {}
loaded_endpoints = []

class Blueprint:
    __endpoints = []

    def __init_subclass__(cls, **kwargs):
        cls.__endpoints = []

        to_get: t.List[t.Union[Websocket, Endpoint]] = pending_endpoints[cls.__name__]
        for ep in to_get:
            func = getattr(cls, ep.__name__)
            ep.callback = func
            cls.__endpoints.append(ep)

    @property
    def endpoints(self) -> t.List[t.Union[Websocket, Endpoint]]:
        return self.__endpoints

    async def on_endpoint_error(self, request, exception_):
        raise exception_


def get_class_and_name(func):
    return func.__qualname__.split(".", maxsplit=1)


def endpoint(route: str, methods: t.Optional[t.List[str]]=None):
    if methods is None:
        methods = ["GET"]

    def wrapper(func):
        if route is None:
            callback_name = func.__name__
        else:
            callback_name = route

        cls, name = get_class_and_name(func)
        if not pending_endpoints.get(cls):
            pending_endpoints[cls] = []
        pending_endpoints[cls].append(Endpoint(func, callback_name, methods))

        return func
    return wrapper


def websocket(route: str):
    def wrapper(func):
        if route is None:
            callback_name = func.__name__
        else:
            callback_name = route

        cls, name = get_class_and_name(func)
        if not pending_endpoints.get(cls):
            pending_endpoints[cls] = []
        pending_endpoints[cls].append(Websocket(func, callback_name))

        return func
    return wrapper

