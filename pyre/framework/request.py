import typing as t


class Request:
    def __init__(
            self,
            route: str,
            parameters: bytes,
            headers: t.List[t.Tuple[bytes, bytes]],
            receive: t.Callable,
    ):
        pass

    def test(self):
        self._x = 1

