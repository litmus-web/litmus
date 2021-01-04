import typing as t


class ConversionException(Exception):
    """ The base class the all converter excepts extend from. """


class ConversionFailure(ConversionException):
    """
    Raised when a converter cannot cannot convert the argument to
    any of the type annotations and there is no default.
    """


class NoDefault:
    """
    A dummy class representing a empty default, due to the nature
    of default values being any type we cannot use None or -1.
    """


def parameter_converter(
        possible_types: list,
        default_return: t.Any,
        cache_handler: t.Optional[t.Callable],
):
    """
    parameter_converter is used for converting annotated parameters
    of a function into the annotated types.

    Conversion is attempted in the order that they are annotated
    e.g: t.Union[int, str] will make the converter first attempt
    to convert the parameter into a integer before attempting to
    convert to a string.
    """
    for type_ in possible_types:
        if isinstance(None, type_):
            possible_types.remove(type_)
            default_return = None

    should_raise = isinstance(default_return, NoDefault)

    def _converter(arg):
        for conv in possible_types:
            try:
                return conv(arg)
            except ValueError:
                continue
        if should_raise:
            raise ConversionFailure(
                f"Cannot convert {arg!r} to any of the types:"
                f" {possible_types!r}"
            )
        return default_return

    if cache_handler is not None:
        return cache_handler(_converter)
    return _converter
