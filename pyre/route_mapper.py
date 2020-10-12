import re
import time
import typing as t
import inspect

_CONVERTER_RE = re.compile("\{([^}]+):([^}]+)\}", re.VERBOSE)

string = "/abc/<x:[^>]+>"


def parse_route(route: str, callback: t.Callable):
    """ parse route asumes the route contains no duplicate `//`
    as the framework should automatically remove them.

    `/foo/bar` would be a acceptable string but `/foo//bar` would not.

    parse_route returns a list containing a tuple with a index number and list
    which can then be handed back to rust upon a url route.
    """
    if route.startswith("/"):
        route = route.lstrip("/")

    converter_matches = _CONVERTER_RE.findall(route)
    subbed_route = _CONVERTER_RE.sub("__FOO_REPLACE__", route)

    split_route = subbed_route.split("__FOO_REPLACE__")

    callback_an = inspect.getfullargspec(callback).annotations
    print(callback_an, split_route)

    output_str = "/"
    for i, (split, converter) in enumerate(zip(split_route, converter_matches)):
        output_str += "{split}{converter}".format(
            split=split,
            converter=f"(?P<{converter[0]}>{converter[1]})"
        )
    print(output_str)
    regex = re.compile(output_str, re.VERBOSE)

    runs = []
    for _ in range(1000):
        start = time.perf_counter()
        regex.match("/abc/owow123")
        stop = time.perf_counter() - start
        runs.append(stop)
    print((sum(runs) / 1000) * 1000)


class Regex:
    pass


def my_route(abc: str, x: Regex):
    pass


parse_route("/abc/{x:[^>]+}", my_route)


class RouteRule:
    def __init__(self, route_part: str, is_regex: bool):
        self.is_regex = is_regex
        self.route_part = route_part

    def match_route(self, section):
        confirm = r"<([^>]:[^>]+)>"
