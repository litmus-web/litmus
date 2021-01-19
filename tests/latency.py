import requests

from time import perf_counter


runs = 1000


times = []
for _ in range(runs):
    start = perf_counter()
    requests.get("http://127.0.0.1:8080")
    stop = perf_counter() - start
    times.append(stop)

print(f"Took {(sum(times) / len(times)) * 1000}ms")