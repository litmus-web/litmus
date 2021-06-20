[![CodeFactor](https://www.codefactor.io/repository/github/project-dream-weaver/litmus/badge)](https://www.codefactor.io/repository/github/project-dream-weaver/litmus)
# Litmus
**NOTE: THIS IS NOT PRODUCTION READY AS OF YET**
A fast asyncronous HTTP server and framework written in Rust for Python.

### Why yet another webserver?
The motivation behind Litmus is to provide a more robust webserver design, unlike Japronto which is only compatible with its own framework; Litmus is ASGI compatible and offers higher performance in raw execution speed without even taking HTTP pipelining or other methods into the equation.

What Litmus certainly isnt, is light weight, Litmus preferes to pool memory and re-use allocation rather than trying to use as little memory as possible, although it would be possible to customise the source code and lower the buffer limits to put Litmus into a lighter memory setting.

### What does Litmus aim to achieve?
Litmus aims to provide a HTTP/1, HTTP/2 and Websocket complient protocol written from scratch and in native level code granting greater security while giving greater performance on both Windows and Unix systems.

### Current state of Litmus:
The main server api has been implemented other than the `H2` and `WS` protocol sections.

### Benchmarks

#### Windows Benchmarks
This is a small test on windows comparing per-worker performance of uvicorn vs litmus using a basic fastapi app and serving the documentation page.

![windows-compo1](https://user-images.githubusercontent.com/57491488/122682851-bb9fc400-d1f3-11eb-8a06-7998b0259db6.png)

