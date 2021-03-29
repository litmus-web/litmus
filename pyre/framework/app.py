import asyncio

from typing import Optional, List, Tuple, Any

from .. import RouterMatcher

from .router import HTTPEndpoint, Blueprint, apply_methods
from .models import Cookies
from .sessions import Session
from .request import HTTPRequest
from .responses import TextResponse, BaseResponse


def _convert_header(data: Tuple[bytes, bytes]) -> Tuple[str, bytes]:
    return data[0].decode('ascii'), data[1]


async def _not_found(send):
    resp = TextResponse("Not Found", status=404)
    p1, p2 = resp.to_raw()
    await send(p1)
    await send(p2)


class App:
    def __init__(self):
        self._endpoints: List[HTTPEndpoint] = []
        self._blueprints: List[Blueprint] = []
        self._matcher = RouterMatcher([])  # re-made later
        self._loop = asyncio.get_event_loop()

    def add_blueprint(self, inst: Blueprint):
        """
        Adds a class instance that inherits from the `framework.Blueprint`
        class, this initiated route methods and adds them to the app.

        Args:
            inst:
                A instance of a framework.Blueprint subclass.
        """
        apply_methods(inst)

        next_id = len(self._blueprints)
        for ep in inst._endpoints:
            ep.id = next_id
            self._endpoints.append(getattr(inst, ep.callback_name))

        self._blueprints.append(inst)

        to_compile = []
        for endpoint in self._endpoints:
            to_compile.append((endpoint.route, endpoint))
        self._matcher = RouterMatcher(to_compile)

    async def __call__(self, scope, receive, send):
        """
        The callable for a ASGI server to invoke.

        Args:
            scope:
                The ASGI scope.

            receive:
                The ASGI receiver callback.

            send:
                The ASGI sender callback.
        """

        if scope['asgi'].get("type"):
            return

        path = scope['path']
        maybe_cb: Optional[Tuple[
            HTTPEndpoint,
            list,
        ]] = self._matcher.get_callback(path)

        if maybe_cb is None:
            await _not_found(scope)
            return

        cb, args = maybe_cb
        args = dict(args)

        query = scope['query_string'].decode()

        headers = list(map(_convert_header, scope['headers']))

        await self.invoke(
            send,
            cb,
            path,
            scope['method'],
            query,
            args,
            headers,
            receive,
            scope.get('client'),
            scope.get('server'),
        )

    async def psgi_app(self, scope, send, receive):
        """
        The PSGI (Pyre Server Gateway Interface) callback handler used
        to interact with the framework.

        Args:
            scope:
                The PSGI scope.
            send:
                The raw PSGI sender callback that needs to be wrapped.

            receive:
                The raw PSGI receiver callback that needs to be wrapped.
        """
        path = scope['path']
        maybe_cb: Optional[Tuple[
            HTTPEndpoint,
            list,
        ]] = self._matcher.get_callback(path)

        async def send_wrapper(result: dict):
            type_ = result['type']
            if type_ == "http.response.start":
                try:
                    send.send_start(
                        result['status'],
                        result['headers'],
                    )
                except BlockingIOError:  # should never happen on start.
                    fut = self._loop.create_future()
                    send.subscribe(fut.set_result)
                    await fut

                    send.send_start(
                        result['status'],
                        result['headers'],
                    )
                return

            elif type_ == "http.response.body":
                try:
                    send.send_body(
                        result.get('more_body', False),
                        result['body'],
                    )
                except BlockingIOError:
                    fut = self._loop.create_future()
                    send.subscribe(fut.set_result)
                    await fut

                    send.send_body(
                        result.get('more_body', False),
                        result['body'],
                    )
                return

            raise TypeError("invalid send type given")

        async def receive_wrapper() -> dict:
            try:
                more_body, data = receive()
            except BlockingIOError:
                fut = self._loop.create_future()
                receive.subscribe(fut.set_result)
                more_body, data = await fut

            return {
                'more_body': more_body,
                'data': data,
            }

        if maybe_cb is None:
            await _not_found(send_wrapper)
            return

        cb, args = maybe_cb
        args = dict(args)

        query = scope['query']
        headers = scope['headers']

        await self.invoke(
            send_wrapper,
            cb,
            path,
            scope['method'],
            query,
            args,
            headers,
            receive_wrapper,
            scope.get('client'),
            scope.get('server'),
        )

    async def invoke(
        self,
        send: Any,
        ep: HTTPEndpoint,
        path: str,
        method: str,
        query: str,
        args: dict,
        headers: List[tuple],
        receive: Any,
        client: Any,
        server: Any,
    ):
        cookies = Cookies.from_raw(headers)
        session = Session(cookies)

        request = HTTPRequest(
            route=path,
            method=method,
            parameters=query,
            url_args=args,
            cookies=cookies,
            session=session,
            receive=receive,
            headers=headers,
            client=client,
            server=server,
        )

        headers = cookies.to_headers()

        bp = self._blueprints[ep.id]
        response: Optional[BaseResponse] = await bp.invoke_endpoint(ep, request)

        if response is None:
            response = TextResponse("Internal Server Error", status=500)

        p1, p2 = response.to_raw(extra_headers=headers)
        await send(p1)
        await send(p2)
