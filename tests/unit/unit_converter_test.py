import inspect

from pyre import framework


def test_compile(func):
    inspection = inspect.getfullargspec(func)
    print(inspection)
    converters = framework._compile_converter(inspection, None)
    print(f"Got converters {converters!r}")
    return func


@test_compile
async def test1():
    ...


@test_compile
async def test2(a, b, c):
    ...


@test_compile
async def test3(a, b: int, c=1):
    ...


@test_compile
async def test4(a, b: int=None, c: str=80347):
    ...


if __name__ == '__main__':
    ...