from typing import Tuple, Optional, List ,Callable


class RouterMatcher:
    """
    Automatically matches a given url to it's callback and
    returns the arguments extracted from it.

    NOTE:
        This is a linting class only, real code is defined in
        the pyre-framework crate.

    Args:
        routes:
            A list of tuples containing a compiled route and
            callback.

    Raises:
        RuntimeError:
            If the regex fails to build Rust will raise a runtime error.
    """

    def __init__(
        self,
        routes: List[Tuple[str, Callable]]  # no cover
    ):
        raise NotImplemented()

    def get_callback(
        self,
        path: str,
    ) -> Optional[Tuple[Callable, List[Tuple[str, str]]]]:
        """
        Gets a callback and arguments if a given url matches a set
        regex, if not None is returned.

        Args:
            path:
                The url path from the http request.

        Returns:
              Either None or a tuple pair containing the callback and a
              list of extracted arguments in a key-value pair of tuples.
        """

        raise NotImplemented()
