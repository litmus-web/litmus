from typing import Tuple, Union
from orjson import dumps


class BaseResponse:
    def __init__(
        self,
        content_type,
        body,
        status,
        headers,
    ):
        self.body: Union[str, bytes] = body or b""
        self.status = status
        self.headers = headers or {}
        self.content_type = content_type

    def to_raw(self) -> Tuple[dict, dict]:
        if isinstance(self.body, str):
            self.body = self.body.encode()

        headers = [
            (b'content-length', len(self.body)),
        ]

        for key, value in self.headers.items():
            headers.append((key.encode(), value.encode()))

        if self.content_type is not None:
            headers.append((
                b'content-type',
                self.content_type.encode(),
            ))

        return (
            {
                'type': 'http.response.start',
                'status': self.status,
                'headers': headers,
            },
            {
                'type': 'http.response.body',
                'body': self.body,
            },
        )


class TextResponse(BaseResponse):
    """
    A pre-set response with content-type set to text/plain

    Args:
        body:
            The body of text to send back.

        status:
            The status code of the response.

        headers:
            Extra headers to send back.
    """

    def __init__(
        self,
        body,
        *,
        status=200,
        headers=None,
    ):
        super().__init__("text/plain", body, status, headers)


class JSONResponse(BaseResponse):
    """
    A pre-set response with content-type set to text/plain

    Args:
        body:
            The body of text to send back.

        status:
            The status code of the response.

        headers:
            Extra headers to send back.
    """

    def __init__(
        self,
        body,
        *,
        status=200,
        headers=None,
    ):
        super().__init__("application/json", dumps(body), status, headers)
