import requests

body = "Hello world" * (64 * 1024 * 1024)
print(len(body))
requests.post("http://127.0.0.1:6060", data=body)