import requests
import time


def timeit(amount=1000):
    def deco(func):
        def wrapper(*args, **kwargs):
            start = time.perf_counter()
            for _ in range(amount):
                func(*args, **kwargs)
            stop = time.perf_counter() - start
            print(f"{amount} iterations took: {round(stop * 1000, 4)}ms, avg: {round((stop / amount )*1000, 4)}ms")
        return wrapper
    return deco


@timeit(amount=10)
def fetch():
    requests.get("http://127.0.0.1:5000")


@timeit(amount=10)
def fetch2():
    requests.get("http://127.0.0.1")


fetch()
fetch2()