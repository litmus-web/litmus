from typing import Optional, List

from .router import HTTPEndpoint, Blueprint, apply_methods
from .models import Cookies
from .request import HTTPRequest

from .. import RouterMatcher


class App:
    def __init__(self):
        self._endpoints: List[HTTPEndpoint] = []
        self._matcher: Optional[None] = None

    def add_blueprint(self, inst: Blueprint):
        apply_methods(inst)

        self._endpoints.extend(inst._endpoints)  # private but needed

        to_compile = []
        for endpoint in self._endpoints:
            to_compile.append((endpoint.route, endpoint))
        self._matcher = RouterMatcher(to_compile)

    async def asgi_app(self, scope, send, receive):
        ...

    async def psgi_app(self, scope, send, receive):
        ...



