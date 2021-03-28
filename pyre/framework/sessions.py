import os

from itsdangerous import URLSafeSerializer

from .models import Cookies

IS_PRODUCTION = os.getenv("DEBUG", False)
SECURE_KEY = os.getenv("SECURE_KEY", None)

if IS_PRODUCTION and SECURE_KEY is None:
    raise EnvironmentError(
        "missing require environment setting 'SECURE_KEY' for sessions, "
        "without this being set you risk your application sessions being"
        "insecure and vulnerable, if you are developing your application you "
        "can bypass this error using 'DEBUG=true' environment key but this is "
        "required for production."
    )
elif SECURE_KEY is None:
    SECURE_KEY = "pyre-development"


class Session(dict):
    def __init__(self, cookies: Cookies, serializer: URLSafeSerializer):
        self._serializer = serializer
        self._should_update = False

        values = cookies.get('session')
        if values is None:
            super().__init__()
            return

        values = serializer.loads(values)
        super().__init__(**values)

    def __setitem__(self, key, value):
        self._should_update = True
        super().__setitem__(key, value)

    def serialize_if_needed(self, cookies: Cookies):
        if not self._should_update:
            return

        values = self._serializer.dumps(self)
        cookies['session'] = values

