"""
This is an example benchmarking test using python to benchmark Litmus one of
my pet projects.

This measures the latency and throughput and displays them with matplotlib.
"""

import matplotlib.pyplot as plt
import sys

from subprocess import Popen, PIPE
from json import loads


def start_benchmark(
    host: str,
    connections: int,
    time: str = "10s",
    rounds: int = 3,
) -> list:
    command = f"rewrk " \
              f"-h {host} " \
              f"-c {connections} " \
              f"-d {time} " \
              f"-t 12 " \
              f"--rounds {rounds} " \
              f"--json"
    process = Popen(command, shell=True, stdout=PIPE, stderr=PIPE)

    out, err = process.communicate()
    out = out.decode(sys.stdin.encoding)
    return [loads(o) for o in out.splitlines()]


def get_avg(inputs: list) -> float:
    return sum(inputs) / len(inputs)


def make_runs():
    host = "http://127.0.0.1:8000/hello/bob"

    x_index = []
    latencies1 = []
    latencies2 = []
    req_secs1 = []
    req_secs2 = []
    for conns in [60, 128, 256, 500]:
        results = start_benchmark(host, conns, time="15s", rounds=3)
        avg_latency = get_avg([o['latency_avg'] for o in results])
        avg_req_sec = get_avg([o['requests_avg'] for o in results])
        print(f"[ {conns} concurrency ]  {avg_latency}ms, {avg_req_sec} req/sec")
        x_index.append(conns)
        latencies1.append(avg_latency)
        req_secs1.append(avg_req_sec)

    host = "http://127.0.0.1:8080/hello/bob"
    for conns in [60, 128, 256, 500]:
        results = start_benchmark(host, conns, time="15s", rounds=3)
        avg_latency = get_avg([o['latency_avg'] for o in results])
        avg_req_sec = get_avg([o['requests_avg'] for o in results])
        print(f"[ {conns} concurrency ]  {avg_latency}ms, {avg_req_sec} req/sec")
        latencies2.append(avg_latency)
        req_secs2.append(avg_req_sec)

    plt.figure()
    plt.xlabel("Connection Concurrency")
    plt.ylabel("Latency / ms")
    plt.title("Benchmark Results")
    plt.plot(x_index, latencies1)
    plt.plot(x_index, latencies2, color="red")
    plt.ylim((0, 200))
    plt.savefig("./pyre_framework_latencies.png")
    plt.close()

    plt.figure()
    plt.xlabel("Connection Concurrency")
    plt.ylabel("Request Per Second")
    plt.title("Benchmark Results")
    plt.plot(x_index, req_secs1)
    plt.plot(x_index, req_secs2, color="red")
    plt.ylim((0, 30_000))
    plt.savefig("./pyre_framework_requests.png")


if __name__ == '__main__':
    make_runs()