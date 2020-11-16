import aiohttp

import time
import asyncio


def timeit(amount=1000):
    def deco(func):
        async def wrapper(*args, **kwargs):
            start = time.perf_counter()
            for _ in range(amount):
                await func(*args, **kwargs)
            stop = time.perf_counter() - start
            print(f"{amount} iterations took: {round(stop * 1000, 4)}ms, avg: {round((stop / amount )*1000, 4)}ms")
        return wrapper
    return deco


async def task(sess):
    async with sess.get("http://127.0.0.1") as r:
        r.raise_for_status()


@timeit(amount=10)
async def fetch2():
    async with aiohttp.ClientSession() as sess:
        await asyncio.gather(*[task(sess) for _ in range(1000)])


asyncio.run(fetch2())