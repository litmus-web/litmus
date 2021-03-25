import matplotlib.pyplot as plt
from typing import List, Tuple

print(plt.style.available)


def get_graph(
    x_label: str,
    y_label: str,
    values: List[Tuple[int, float]],
    values2: List[Tuple[int, float]]
):

    plt.figure()

    x = [v[0] for v in values]
    y = [v[1] for v in values]
    plt.bar(x, y, width=20)

    x = [v[0] for v in values2]
    y = [v[1] for v in values2]
    plt.bar(x, y, color='red', width=20)

    plt.xlabel(x_label)
    plt.ylabel(y_label)
    plt.title("Benchmark Results")

    plt.show()


if __name__ == '__main__':
    get_graph("Concurrency", "Req/Sec", [
        (60, 9974.33),
        (128, 12264.13),
        (256, 15039.14),
        (512, 13564.85),
    ],
    [
        (60, 0),
        (128, 9170.91),
        (256, 8868.60),
        (512, 8406.54),
    ]
  )
