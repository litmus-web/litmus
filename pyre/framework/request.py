import typing as t
from orjson import loads

from .models import Parameters, Headers, Cookies
from .sessions import Session


class BaseRequest:
    """
    A base request instance that all other request types inherit off.

    This implements all the basic level handling like header access,
    routes, parameters etc...

    Args:
        route:
            The url path section of the HTTP request.

        parameters:
            The raw url query parameters given as a un-decoded set of bytes.

        headers:
            A iterable of key-value pairs of headers where the key is a string
            and the value is bytes (following the HTTP spec)

        receive:
            The asgi callback in order to receive data from the server.

        server:
            The server host and port.

        client:
            The remote client host and port.
    """
    def __init__(
        self,
        route: str,
        parameters: bytes,
        headers: t.List[t.Tuple[str, bytes]],
        receive: t.Callable,
        server: t.Tuple[str, int],
        client: t.Tuple[str, int],
    ):
        self._path = route
        self._query = Parameters.from_raw(parameters)
        self._headers = Headers.from_raw(headers)
        self._server = server
        self._client = client

        self.__more_body = True
        self.__receive = receive

    @property
    def path(self):
        """ The path section of the request url """
        return self._path

    @property
    def query(self) -> Parameters:
        """ The query parameters of the request url """
        return self._query

    @property
    def headers(self) -> Headers:
        """ The headers of the request """
        return self._headers

    @property
    def server_info(self) -> t.Tuple[str, int]:
        """ The server's ip and port """
        return self._server

    @property
    def remote_address(self) -> t.Tuple[str, int]:
        """ The client's ip and port """
        return self._client

    async def read(self) -> bytes:
        """
        Reads a set amount of data from the body of the request.

        Returns:
            A buffer of bytes containing the data read.

        Raises:
            A IOError if there is no more data to be read from the socket.
        """

        if not self.__more_body:
            raise IOError("no more data can be read from this request")

        recv = await self.__receive()

        self.__more_body = recv['more_body']

        return recv['data']


class HTTPRequest(BaseRequest):
    """
    A standard HTTP request.

    This request extends the base request with sessions, cookies and
    reading methods for quicker and easier parsing of data.

    Args:
        route:
            The url path section of the HTTP request.

        cookies:
            The cookies of the given http request.

        session:
            The the session of the given http request.

        parameters:
            The raw url query parameters given as a un-decoded set of bytes.

        headers:
            A iterable of key-value pairs of headers where the key is a string
            and the value is bytes (following the HTTP spec)

        receive:
            The asgi callback in order to receive data from the server.

        server:
            The server host and port.

        client:
            The remote client host and port.
    """
    def __init__(
        self,
        route: str,
        parameters: bytes,
        cookies: Cookies,
        session: Session,
        headers: t.List[t.Tuple[str, bytes]],
        receive: t.Callable,
        server: t.Tuple[str, int],
        client: t.Tuple[str, int],
    ):
        self._cookies = cookies
        self._session = session

        super().__init__(route, parameters, headers, receive, server, client)

    @property
    def cookies(self) -> Cookies:
        """ The request cookies """
        return self._cookies

    @property
    def session(self) -> Session:
        """ The request session """
        return self._session

    async def _read_all(self) -> bytes:
        buffer = b""
        while True:
            try:
                data = await self.read()
            except IOError:
                return buffer

            buffer += data

    async def text(self, *, encoding="utf-8") -> str:
        """
        Reads all data from the request and decodes it to a str.

        Note this is generally not a good idea to do on unknown sized requests.
        Instead it is advised to use `text_iter()` which allows reasonable
        chunking of data.

        Args:
            encoding:
                The encoding type of the data, by default this is 'utf-8'

        Returns:
            A string of the given data.
        """
        buffer = await self._read_all()
        return buffer.decode(encoding)

    async def json(self) -> t.Any:
        """
        Reads all data from the request and decodes it with 'loads'.

        Note this is generally not a good idea to do on unknown sized requests.
        Instead it is advised to use `text_iter()` which allows reasonable
        chunking of data.

        Returns:
            A loaded value of the given json data.
        """

        buffer = await self._read_all()
        return loads(buffer)

    async def text_iter(self) -> bytes:
        """
        Reads data in chunks from the request yielding chunks
        until the data is exhausted.
        """

        while True:
            try:
                data = await self.read()
            except IOError:
                return

            yield data
