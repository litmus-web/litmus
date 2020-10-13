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


class RouteError(Exception):
    """ A base route error that gets raised by the route compiler """


class KeywordRouteError(RouteError):
    """ Raised when a keyword the route builder uses has been entered as a route """


class PathNotLastError(RouteError):
    """ Raised when the 'path' converter is used in the middle of a route invalidating the regex """


class ConverterEmpty(RouteError):
    """ Raised when the converter is left blank """


def parse_route(route_str: str):
    """ parse route asumes the route contains no duplicate `//`
    as the framework should automatically remove them.

    `/foo/bar` would be a acceptable string but `/foo//bar` would not.

    parse_route returns a list containing a tuple with a index number and list
    which can then be handed back to rust upon a url route.
    """
    if "__FOO_REPLACE__" in route_str:
        raise KeywordRouteError(f"Route: {route_str!r} cannot contain a reserved keyword.")

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
            raise ConverterEmpty(f"Parameter {converter[1]!r} converter type cannot be empty.")

        if path_exists:
            raise PathNotLastError(
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
    return x, stop * 100


def make_route_str(combination: tuple, seperator: str):
    route_str = "/"
    for letter, com in zip("abcdefghijklmnopqrstuv", combination):
        route_str += f"{seperator}/"
        route_str += f"{{{letter}:{com}}}/"
    return route_str


def make_pass_string(combination: tuple, seperator: str):
    good_converter = {
        "alpha": r"foo",
        "alnum": r"f00",
        "string": r"h3llo-w0rld",
        "int": r"13058",
        "path": r"world/hello.txt",
        "uuid": r"6a2f41a3-c54c-fce8-32d2-0324e1c32e22"
    }

    good = "/"
    for com in combination:
        success_str = good_converter[com]
        good += f"{seperator}/{success_str}/"
    return good


def make_fail_string(combination: tuple, seperator: str):
    bad_converter = {
        "alpha": r"f0o",
        "alnum": r"f00-bbsf",
        "string": r"h3l/lo-/w0rld",
        "int": r"13A58",
        "path": r"world/hello.txt",
        "uuid": r"6a2f41afa3-c54c-fsfce8-32d2-0324ea1c32e22"
    }

    bad = "/"
    for com in combination:
        success_str = bad_converter[com]
        bad += f"{seperator}/{success_str}/"
    return bad


if __name__ == '__main__':
    import itertools

    combinations = itertools.permutations(_standard_type_converter.keys(), len(_standard_type_converter.keys()))

    passed = True
    times, count = [], 0
    for i, combo in enumerate(combinations):
        route = make_route_str(combo, "abc")
        pass_str = make_pass_string(combo, "abc")
        fail_str = make_fail_string(combo, "abc")

        try:
            built_regex = parse_route(route)
        except PathNotLastError:
            if combo.index("path") == (len(combo) - 1):
                print(f"Test Failed! - Path should have succeeded.\n"
                      f" Combo: {combo!r}\n"
                      f" Route: {route!r}\n"
                      f" ")
                passed = False
                break
            continue

        matched, timed = test_regex(built_regex, pass_str)
        if not matched:
            print(f"Test Failed! - Route should have matched\n"
                  f" Combo: {combo!r}\n"
                  f" Route: {route!r}\n"
                  f" Built Route: {built_regex!r}\n"
                  f" Pass Str: {pass_str!r}\n")
            passed = False
            break
        times.append(timed)

        matched, timed = test_regex(built_regex, pass_str)
        if not matched:
            print(f"Test Failed! - Route should have failed to match\n"
                  f" Combo: {combo!r}\n"
                  f" Route: {route!r}\n"
                  f" Built Route: {built_regex!r}\n"
                  f" Fail Str: {fail_str!r}\n")
            passed = False
            break
        times.append(timed)
        count = i + 1

    if passed:
        print(f"All Test OK! - Ran {count} tests.\n"
              f" Avg Latency: {(sum(times) / len(times))*1000}ms\n")
    else:
        print(f"Test failed on iteration: {count}")
