import asyncio
import typing as t


class PyreProtocol(asyncio.Protocol):
    """
    The Rust-built pyre protocol, this is what the factory
    should produce for `EventLoop.create_server` and takes
    """

    def __init__(self, callback: t.Callable):
        ...


class ASGIRunner:
    """ Pyre's ASGI callback manager. """

    def __init__(
            self,
            callback: t.Callable,
            future: t.Callable,

            method: str,
            raw_path: bytes,
            headers: t.Dict[bytes, bytes]
    ): ...
