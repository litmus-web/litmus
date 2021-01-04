import inspect
import re
import typing as t

from .converters import parameter_converter, NoDefault

__all__ = [
    "Blueprint",
    "HTTPEndpoint",
    "BaseEndpoint",
    "endpoint",
]


_converter_re = re.compile(r"\{([^}]+):([^}]+)}", re.VERBOSE)

_standard_type_re_converter = {
    "alpha": r"[A-Za-z]+",
    "alnum": r"[A-Za-z0-9]+",
    "string": r"[^\/]*",
    "int": r"[0-9]+",
    "path": r".*",
    "uuid": r"\b[0-9a-f]{8}\b-[0-9a-f]{4}-[0-9a-f]"
            r"{4}-[0-9a-f]{4}-\b[0-9a-f]{12}\b"
}


def parse_route(route_str: str) -> str:
    """ parse route assumes the route contains no duplicate `//`
    as the framework should automatically remove them.

    `/foo/bar` would be a acceptable string but `/foo//bar` would not.

    parse_route returns a string with a compiled regex pattern to be used for
    matching upon a request.
    """
    if "__FOO_REPLACE__" in route_str:
        raise ValueError(
            f"Route: {route_str!r} cannot contain a reserved keyword.")

    # Matches anything in the `{variable:converter}` pattern and
    # gives us the variable name and converter name in the form of
    # a tuple, this gets matched later on.
    converter_matches = _converter_re.findall(route_str)

    # We just need some string that has a very low chance of being
    # used in production for a route to then split off.
    subbed_route = _converter_re.sub("__FOO_REPLACE__", route_str)
    split_route = subbed_route.split("__FOO_REPLACE__")

    output_regex, path_exists = "", False
    for split, converter in zip(split_route, converter_matches):
        if converter[1] == "":
            raise ValueError(
                f"Parameter {converter[1]!r} converter type cannot be empty.")

        if path_exists:
            raise ValueError(
                "Url cannot have anything following after a 'path' converter.\n"
                "If you are attempting to match anything other than '/' "
                "use the 'string' converter"
            )

        converter_re = _standard_type_re_converter.get(
            converter[1].lower(),
            converter[1]
        )
        output_regex += f"{split}(?P<{converter[0]}>{converter_re})"

        if converter[1].lower() == "path":
            path_exists = True

    return output_regex


def _make_converter(
        annotation: t.Any,
        default: t.Any,
        cache_handler: t.Callable,
):
    """
    _make_converter produces a Callable type object that when called
    will take a single argument and attempt to convert it to the typed
    annotations.

    If 'annotation' is None then the converter will return
    the value it is given.
    """
    if annotation is None:
        return lambda a: a

    # Check for typing annotations from the 'typing' module.
    if hasattr(annotation, "__origin__"):
        if annotation.__origin__ not in (t.Union, t.Optional):
            raise TypeError(
                f"Converter only supports 'typing.Union' or 'typing.Optional'"
                f" annotations from the typing module, found "
                f"{annotation.__origin__!r}.\n Use a custom converter if you "
                f"require this conversion.")

        possible_converters = list(annotation.__args__)
    else:
        possible_converters = [annotation]

    return parameter_converter(
        possible_converters,
        default,
        cache_handler
    )


def _compile_converter(
        inspection: inspect.FullArgSpec,
        cache_handler: t.Callable,
):
    """
    _compile_converter works by taking in the inspected function
    result and a cache_handler (the cache_handler may be a NoneType)

    The inspection arguments are reversed as to allow the system to map
    the default values back to their correct parameters.

    As the converters are added in reverse order of the function's
    parameters we simply reverse the list at the end to correct it.
    """

    converters_reversed = []
    for i, arg_name in enumerate(inspection.args[::-1]):
        annotation = inspection.annotations.get(arg_name)
        if inspection.defaults is not None and i < len(inspection.defaults):
            default = inspection.defaults[::-1][i]
        else:
            default = NoDefault()

        converters_reversed.append(
            _make_converter(
                annotation,
                default,
                cache_handler,
            )
        )

    return converters_reversed[::-1]


class BaseEndpoint:
    """
    The BaseEndpoint class is responsible for basic handling of a Blueprint
    route, this handles making the compiled route and converters as well as
    applying the converters at runtime when the endpoint is called.

    This contains the basic components needed to build a basic callback endpoint
    this should not be used as a standard endpoint but be inherited in order
    to make standard endpoints or websocket endpoints.
    """

    def __init__(
            self,
            route: str,
            callback: t.Callable,
            before_invoke: t.Optional[t.Callable],
            on_error: t.Optional[t.Callable],
            converter_cache: t.Callable,
            **options,
    ):
        self.route_options = options

        self.callback_name = callback.__name__
        self.callback = callback
        self.before_invoke = before_invoke
        self.on_error = on_error

        self._converter_cache = converter_cache

        callback_inspect = inspect.getfullargspec(callback)
        self._converters = _compile_converter(callback_inspect, converter_cache)

        self._raw_route = route
        self._compiled_route = parse_route(route)

    async def __call__(self, request, *args):
        try:
            if self.before_invoke is not None:
                await self.before_invoke(request, *args)
            new_args = map(self._convert, zip(args, self._converters))
            return await self.callback(request, *new_args)
        except Exception as e:
            if self.on_error is not None:
                await self.on_error(request, e)
                return e
            raise e

    @staticmethod
    def _convert(parts):
        return parts[1](parts[0])

    @property
    def route(self):
        return self._compiled_route


class HTTPEndpoint(BaseEndpoint):
    def __init__(
            self,
            route: str,
            callback: t.Callable,
            before_invoke: t.Optional[t.Callable] = None,
            on_error: t.Optional[t.Callable] = None,
            converter_cache: t.Callable = None,
            **options
    ):
        """
        The main HTTP endpoint for standard routes. This is is created when
        ever a function is decorated with the `@pyre.framework.endpoint()`
        decorator.

        Note that the endpoint is not created until the class instance is
        actually methods created (If it is a class blueprint) due to the
        nature of python instance.

        args:
            route:
                The raw route of the endpoint using the framework
                placeholders e.g. 'hello/world/{name:alpha}'

            callback:
                This callable should be a coroutine type and will be called
                when ever a in coming request's URL matches the route.

            before_invoke:
                This is a Optional callable that should be a coroutine and
                will be called before any arguments are converted and the
                endpoint called.

            on_error:
                This is a Optional callable that should be a coroutine,
                if this is is not None it will be called when ever the
                endpoint raises an exception, *this will silence the error
                if not re-raised*.

            converter_cache:
                This is a Optional callable that can be something like
                functool's lru_cache() system or another custom cache
                system, this can be used to save time when processing
                expensive but repetitive inputs or converter operations.

            **options:
                Any other options you wish to be sent to the route add
                function on the framework WebApplication instance.
        """

        super().__init__(
            route,
            callback,
            before_invoke,
            on_error,
            converter_cache,
            **options
        )

    def __repr__(self):
        return f"Endpoint(" \
               f"name={self.callback_name!r}, " \
               f"raw_route={self._raw_route!r}, " \
               f"compiled_route={self.route!r})"


class Blueprint:
    _endpoints = []

    def __init_subclass__(cls, **kwargs):
        cls._endpoints = []

        for k, v in cls.__dict__.items():
            if k.startswith("__") or k.endswith("__"):
                continue

            if isinstance(v, HTTPWrapper):
                cls._endpoints.append(v)
                setattr(cls, v.callback_name, v.original_callback)

    async def invoke_endpoint(self, ep, request):
        try:
            return await ep(request, *request.args)
        except Exception as e:
            await self.on_blueprint_error(request, e)

    async def on_blueprint_error(self, request, exception):
        raise exception


class HTTPWrapper:
    __error_name = ""
    __middle_name = ""

    def __init__(self, route, callback, **kwargs):
        self.route = route
        self.kwargs = kwargs
        self.callback_name = callback.__name__
        self.original_callback = callback

    def to_endpoint(self, instance):
        callback = getattr(instance, self.callback_name)
        error_handler = getattr(instance, self.__error_name, None)
        before_invoke = getattr(instance, self.__middle_name, None)

        return HTTPEndpoint(
            route=self.route,
            callback=callback,
            before_invoke=before_invoke,
            on_error=error_handler,
            **self.kwargs
        )

    @classmethod
    def error(cls, func):
        cls.__error_name = func.__name__
        return func

    @classmethod
    def before_invoke(cls, func):
        cls.__middle_name = func.__name__
        return func


def endpoint(route: str, **kwargs):
    def wrapper(func):
        return HTTPWrapper(route, func, **kwargs)

    return wrapper


def apply_methods(instance):
    endpoints: t.List[HTTPWrapper] = instance._endpoints
    for ep in endpoints:
        setattr(instance, ep.callback_name, ep.to_endpoint(instance))
