[![CodeFactor](https://www.codefactor.io/repository/github/litmus-web/litmus/badge)](https://www.codefactor.io/repository/github/litmus-web/litmus)
# Litmus
A fast asynchronous HTTP server and framework written in Rust for Python. To live and die by speed.

## WARNING
This project is intermittently maintained i.e as and when I have some free time to do so. Generally, the LNX project and Lust project take precedence.

## Should I use this server over the existing systems?
Probably not, sure Litmus benches considerably faster than uvicorn with your typical small, plain text response benchmarks (generally 70%+ throughput increase) as soon as you put this into a real world situation I doubt you're going to get over 10% increase, you will probably get lower average latency but again by margins of around 10% so pick and choose your poison.

### Current state of Litmus:
The main server api has been implemented other than the `H2` and `WS` protocol sections.
