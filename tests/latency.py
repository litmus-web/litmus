import requests

from time import perf_counter


runs = 1000


def timeit(host: str):
    times = []
    for _ in range(runs):
        start = perf_counter()
        requests.get(host)
        stop = perf_counter() - start
        times.append(stop)

    print(f"Host: {host} Took {(sum(times) / len(times)) * 1000}ms")


timeit("http://127.0.0.1:8080")
timeit("http://127.0.0.1:5000")