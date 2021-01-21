import time

def iterate():
    a = 0
    for _ in range(1_000_000_000):
        a += 1

start = time.perf_counter()
for _ in range(500):
    iterate()
stop = time.perf_counter() - start
print(stop)
