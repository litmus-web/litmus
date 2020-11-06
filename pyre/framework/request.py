import typing as t


class Request:
    def __init__(
            self,
            route: str,
            parameters: bytes,
            headers: t.List[t.Tuple[bytes, bytes]],
            receive: t.Callable,
    ):

    def test(self):
        self._x = 1


x = Request()
x.test()
