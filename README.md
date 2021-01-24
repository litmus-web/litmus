[![CodeFactor](https://www.codefactor.io/repository/github/project-dream-weaver/pyre/badge)](https://www.codefactor.io/repository/github/project-dream-weaver/pyre)
[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/Project-Dream-Weaver/Pyre)](https://rust-reportcard.xuri.me/report/github.com/Project-Dream-Weaver/Pyre)

# Pyre
**NOTE: THIS IS NOT PRODUCTION READY AS OF YET**
A fast asyncronous HTTP server and framework written in Rust for Python.

### Why yet another webserver and framework?
The motivation behind Pyre is to provide a more robust webserver design, unlike Japronto which is only compatible with its own framework; Pyre is ASGI compatible and offers higher performance in raw execution speed without even taking HTTP pipelining or other methods into the equation.

What Pyre certainly isnt, is light weight, Pyre preferes to pool memory and re-use allocation rather than trying to use as little memory as possible, although it would be possible to customise the source code and lower the buffer limits to put Pyre into a lighter memory setting.

The framework side of Pyre is designed to add a more OOP and event driven framework rather than yet another Flask copy, this will also be written with a Rust backbone and lazy evaluation to try and make each request as light weight as possible.


### What does Pyre aim to achieve?
Pyre aims to provide a HTTP/1, HTTP/2 and Websocket complient protocol written from scratch and in native level code granting greater security while giving greater performance on both Windows and Unix systems.

### Current state of Pyre:
The main server api has been implemented other than the direct protocols, these will become the `H1`, `H2` and `WS` protocol sections, this may seem like alot to do and it is however alot of the code base is re-implementing / re-creating the asyncio streams api to be more rust friendly and high performance.

### Benchmarks

#### Pre-Alpha Benchmarks
**Note these benchmarks were taken back in the pre-aplha builds before the most recent set of refractors**

These benchmarks were taken while testing the pre-alpha without HTTP/1 Pipelining as neither servers support concurrent pipelining using `wrk`, the comparision was Pyre VS Uvicorn which is the current go to performance server.

![alt text](https://github.com/Project-Dream-Weaver/Pyre/blob/main/images/bench-pre-alpha.png "Pyre Benchmarks")
