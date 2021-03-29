from typing import Optional, List, Tuple, Any

from .. import RouterMatcher

from .router import HTTPEndpoint, Blueprint, apply_methods
from .models import Cookies
from .sessions import Session
from .request import HTTPRequest
from .responses import TextResponse, BaseResponse


def _convert_header(data: Tuple[bytes, bytes]) -> Tuple[str, bytes]:
    return data[0].decode('ascii'), data[1]


async def app(scope, receive, send):
    print("wew")


class App:
    def __init__(self):
        self._endpoints: List[HTTPEndpoint] = []
        self._blueprints: List[Blueprint] = []
        self._matcher = RouterMatcher([])  # re-made later

    def add_blueprint(self, inst: Blueprint):
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
        await self.asgi_app(scope, receive, send)

    async def asgi_app(self, scope, receive, send):
        if scope['asgi'].get("type"):
            return

        path = scope['path']
        maybe_cb: Optional[Tuple[
            HTTPEndpoint,
            list,
        ]] = self._matcher.get_callback(path)

        if maybe_cb is None:
            resp = TextResponse("Not Found", status=404)
            p1, p2 = resp.to_raw()
            await send(p1)
            await send(p2)
            return

        cb, args = maybe_cb
        args = dict(args)

        query = scope['query_string'].decode()

        headers = list(map(_convert_header, scope['headers']))

        await self.invoke(
            send,
            cb,
            path,
            query,
            args,
            headers,
            receive,
            scope.get('client'),
            scope.get('server'),
        )

    async def psgi_app(self, scope, send, receive):
        ...

    async def invoke(
        self,
        send: Any,
        ep: HTTPEndpoint,
        path: str,
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




