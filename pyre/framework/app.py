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

        if not route.startswith("/"):
            route = f"/{route}"



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
        pass

