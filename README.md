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


### Reduction of Latency
![image](https://user-images.githubusercontent.com/57491488/112849025-92621280-90a0-11eb-96a2-f69aa3618252.png)

As you can see in the above imade with Uvicorn(blue) and Pyre(red) I achieve an average reduction of latency by around 50% which is over my 40% goal.

### Increase Throughput
![image](https://user-images.githubusercontent.com/57491488/112848941-81b19c80-90a0-11eb-9017-91c44570a39c.png)

In the above image like my latency reduction pyre (red) is around 50% higher throughput than uvicorn (blue) which is again higher than my original 40% goal.


