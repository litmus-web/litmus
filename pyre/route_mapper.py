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
    "uuid": r"\b[0-9a-f]{8}\b-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-\b[0-9a-f]{12}\b"
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

    output_regex, path_exists = "", False
    for i, (split, converter) in enumerate(zip(split_route, converter_matches)):
        if converter[1] == "":
            raise ValueError(f"Parameter {converter[1]!r} converter type cannot be empty.")

        if path_exists:
            raise ValueError(
                "Url cannot have anything following after a 'path' converter.\n"
                "If you are attempting to match anything other than '/' use the 'string' converter"
            )

        converter_re = _standard_type_converter.get(converter[1].lower(), converter[1])
        output_regex += f"{split}(?P<{converter[0]}>{converter_re})"

        if converter[1].lower() == "path":
            path_exists = True

    return output_regex


def test_regex(regex_str: str, test_string: str):
    regex = re.compile(regex_str, re.VERBOSE)
    start = time.perf_counter()
    x = regex.fullmatch(test_string) is not None
    stop = time.perf_counter() - start
    print(f"Regex Matched: {x}")
    print(f"Time Taken: {stop * 1000}ms")


if __name__ == '__main__':
    import itertools

    test_paths = [
        "abc",
        "foo",
        "",
        "bar",
        "delta123",
        "1234",
        "0",
        "0-012-2452-2342",
        "6a2f41a3-c54c-fce8-32d2-0324e1c32e22",
        "file.txt"
    ]
    test_paths.extend(_standard_type_converter.keys())
    combinations = itertools.permutations(test_paths, 5)
    for combo in combinations:
        pass

    built_regex = parse_route("/abc/{p:uuid}")
    test_regex(built_regex, "/abc/981a-2341/owow12/ayacuac.txt")
