import re
import time
import typing as t
import inspect

__all__ = ["parse_route"]

_converter_re = re.compile("\{([^}]+):([^}]+)\}", re.VERBOSE)

_standard_type_converter = {
    "alpha": r"[A-Za-z]+",
    "alnum": r"[A-Za-z0-9]+",
    "string": r"[^\/]*",
    "int": r"[0-9]+",
    "path": r".*",
    "uuid": r""
}


def parse_route(route: str):
    """ parse route asumes the route contains no duplicate `//`
    as the framework should automatically remove them.

    `/foo/bar` would be a acceptable string but `/foo//bar` would not.

    parse_route returns a list containing a tuple with a index number and list
    which can then be handed back to rust upon a url route.
    """
    if "__FOO_REPLACE__" in route:
        raise ValueError(f"Route: {route!r} cannot contain a reserved keyword.")

    # Matches anything in the `{variable:converter}` pattern and
    # gives us the variable name and converter name in the form of
    # a tuple, this gets matched later on.
    converter_matches = _converter_re.findall(route)

    # We just need some string that has a very low chance of being
    # used in production for a route to then split off.
    subbed_route = _converter_re.sub("__FOO_REPLACE__", route)

    split_route = subbed_route.split("__FOO_REPLACE__")

    should_raise = False
    output_str = ""
    for i, (split, converter) in enumerate(zip(split_route, converter_matches)):
        if should_raise:
            raise ValueError(
                "Url cannot have anything following after a 'path' converter.\n"
                "If you are attempting to match anything other than '/' use the 'string' converter"
            )

        converter_re = _standard_type_converter.get(converter[1].lower(), converter[1])
        output_str += "{split}{converter}".format(
            split=split,
            converter=f"(?P<{converter[0]}>{converter_re})"
        )

        if converter[1].lower() == "path":
            should_raise = True

    print(output_str)
    regex = re.compile(output_str, re.VERBOSE)

    start = time.perf_counter()
    x = regex.match("/abc/981a-2341/owow123") is not None
    stop = time.perf_counter() - start
    print(x)
    print(stop * 1000)


parse_route("/abc/{p:string}/{x:[^>]+}")
