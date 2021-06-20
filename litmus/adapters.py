from asyncio import get_running_loop


class LSGIToASGIAdapter:
    def __init__(self, app):
        self._loop = get_running_loop()
        self._app = app

    async def __call__(self, scope, send, receive):
        """
        The LSGI (Litmus Server Gateway Interface) callback handler used
        to interact with the framework.

        Args:
            scope:
                The LSGI scope.
            send:
                The raw LSGI sender callback that needs to be wrapped.

            receive:
                The raw LSGI receiver callback that needs to be wrapped.
        """

        scope['query_string'] = scope['query'].encode()
        scope['raw_path'] = scope['path'].encode()
        scope['asgi'] = {'spec_version': '2.1', 'version': '3.0'}
        scope['headers'] = list(map(
            lambda item: (item[0].encode(), item[1]),
            scope['headers'],
        ))

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

        await self._app(scope, receive_wrapper, send_wrapper)
