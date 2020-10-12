import typing as t


class WebApplication:
    def __init__(self):
        self._routes = {}

    def add_route(
            self,
            route: str,
            callback: t.Callable,
            methods=None,
            cache_response=False,
    ) -> None:
        if methods is None:
            methods = {"GET"}
        else:
            methods = set(methods)

        if route.startswith("/"):
            route = route.lstrip("/")

        # We dont want to handle duplicated `//` as it'll break the next part
        while "//" in route:
            route = route.replace("//", "/")

        route_parts = route.split("/")






class AppRunner:
    def __init__(
            self,
            app: t.Callable,
            host: str = "127.0.0.1",
            port: int = 8000,
            max_headers: int = 32,
    ):
        self._app = app
        self._host = host
        self._port = port
        self._max_headers = max_headers

    def run(self):

