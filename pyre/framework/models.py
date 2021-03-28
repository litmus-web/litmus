from typing import List, Union, Tuple, Optional, Any
from dataclasses import dataclass
from functools import cache


@dataclass(frozen=True, repr=True)
class FrozenCollection(dict):
    _all: dict

    @classmethod
    def from_raw(cls, data) -> "FrozenCollection":
        raise NotImplemented()

    def get(self, key) -> Optional[Any]:
        maybe_value = self._all.get(key)

        if maybe_value is None:
            return None

        return maybe_value.decode()

    def __getitem__(self, item) -> Any:
        return self._all[item].decode()


class Parameters(FrozenCollection):
    """
    A frozen / immutable collection of key-value pairs for url query
    parameters.
    """

    @classmethod
    def from_raw(cls, data: bytes) -> "Parameters":
        """
        Creates a new instance of this class from a given raw string.

        Args:
            data:
                The raw data to be parsed into the separate key-value
                pairs.

        Returns:
            A new instance of Parameters which are frozen.
        """

        parts = {}
        pairs = data.split(b"&")
        for pair in pairs:
            key, value = pair.split(b"=", maxsplit=1)
            parts[key] = value

        return cls(
            _all=parts
        )

    def get(self, key: str) -> Optional[str]:
        """
        Attempts to get a value of a given key that may or may not exist.

        If the key does exist the value is decoded and returned, otherwise
        None is returned.

        Args:
            key:
                The key to match the given query parameters.

        Returns:
            Either a string or None depending on if the value was found or
            not.
        """

        maybe_value = self._all.get(key)

        if maybe_value is None:
            return None

        return maybe_value.decode()

    def __getitem__(self, item) -> str:
        return self._all[item].decode()


class Headers(FrozenCollection):
    """
    A frozen / immutable collection of headers
    """

    @classmethod
    def from_raw(cls, headers: List[Tuple[str, bytes]]) -> "Headers":
        """
        Creates a new instance of this class from a given set of raw
        headers following the type hints.

        Args:
            headers:
                The raw set of headers following a string key and bytes
                value pattern.

        Returns:
            A new instance of the Headers class set to be frozen.
        """

        parts = {}
        for key, value in headers:
            if key not in parts:
                parts[key] = value
                continue

            temp = parts[key]

            if isinstance(temp, tuple):
                parts[key] = (*temp, value)
            else:
                parts[key] = (value,)

        return cls(
            _all=parts
        )

    def get(self, key: str) -> Optional[Union[bytes, Tuple[bytes, ...]]]:
        """
        Attempts to get a value of a given key that may or may not exist.

        If the key does exist the value is decoded and returned, otherwise
        None is returned.

        Args:
            key:
                The key to match the given header key.

        Returns:
            Either a None or bytes if one value belongs to that key or a tuple
            of bytes if they key has many values, e.g. cookies.
        """

        return super().get(key)

    @cache
    def get_one(self, key: str) -> Optional[bytes]:
        """
        Attempts to get a value of a given key that may or may not exist.

        This function is nearly identical to `get()` however it only returns a
        single value if the header key belongs to multiple values.

        Args:
            key:
                The key to match the given header key.

        Returns:
            Either a None or bytes if one value belongs to that key.
        """

        maybe_value = self._all.get(key)

        if maybe_value is None:
            return None

        if isinstance(maybe_value, tuple):
            return maybe_value[0]
        return maybe_value

    def __getitem__(self, item) -> Union[bytes, Tuple[bytes, ...]]:
        return super().__getitem__(item)


def _get_cookie_str(headers: List[Tuple[str, bytes]]) -> Optional[bytes]:
    for key, value in headers:
        if key.lower() == "cookie":
            return value
    return None


class Cookies(dict):
    def __init__(self, _all: dict):
        self._all = _all

        super().__init__(**_all)

    def to_headers(self) -> List[Tuple[bytes, bytes]]:
        headers = []
        for key, value in self.items():
            headers.append((
                b"Set-Cookie",
                b"%s=%s" % (key, value)
            ))

        return headers

    @classmethod
    def from_raw(cls, headers: List[Tuple[str, bytes]]) -> "Cookies":
        """
        Creates a new instance of cookies from a given set of headers.

        Args:
            headers:
                The initial set of headers given via the request scope.

        Returns:
            A new instance of Cookies which are frozen.
        """

        cookie_str = _get_cookie_str(headers)

        if cookie_str is None:
            return cls(_all={})

        parts = {}
        pairs = cookie_str.split(b"; ")
        for pair in pairs:
            key, value = pair.split(b"=", maxsplit=1)
            parts[key.decode()] = value

        return cls(
            _all=parts
        )

