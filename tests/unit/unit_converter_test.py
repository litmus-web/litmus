import inspect

from pyre import framework


def test_compile(func):
    inspection = inspect.getfullargspec(func)
    converters = framework._compile_converter(inspection, None)
    print(f"Got converters {converters!r}")
    return func


@test_compile
async def test1():
    ...


@test_compile
async def test2():
    ...


@test_compile
async def test3():
    ...


@test_compile
async def test4():
    ...


if __name__ == '__main__':
    ...