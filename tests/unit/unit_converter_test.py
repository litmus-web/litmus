import asyncio
import inspect

from pyre import framework


def test_compile(func):
    inspection = inspect.getfullargspec(func)
    converters = framework._compile_converter(inspection, None)

    async def wrapper(*args):
        new_args = map(lambda item: item[0](item[1]), zip(converters, args))
        return await func(*new_args)
    return wrapper


@test_compile
async def test1():
    return []


@test_compile
async def test2(a, b, c):
    return [a, b, c]


@test_compile
async def test3(a, b: int, c=1):
    return [a, b, c]


@test_compile
async def test4(a, b: int=None, c: str=80347):
    return [a, b, c]


async def main():
    assert await test1() == [], "test1 failed"
    print("Test 1 Passed!")

    assert await test2(1, "b", None) == [1, "b", None], "test2 failed"
    print("Test 2 Passed!")

    assert await test3("abc", "123", "d") == ["abc", 123, "d"], "test3 failed"
    print("Test 3 Passed!")

    assert await test4(12, "a23", 132) == [12, None, "132"], "test4 failed"
    print("Test 4 Passed!")


if __name__ == '__main__':
    asyncio.run(main())