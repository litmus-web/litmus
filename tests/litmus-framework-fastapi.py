import asyncio
import litmus
import uvloop

from fastapi import FastAPI

uvloop.install()

litmus.init_logger("info", None, True)

# asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

app = FastAPI()
server = None


@app.get("/stats")
async def show_stats():
    print(server._server.len_clients())


@app.get("/hello")
async def hello_world():
    return "hello, world"


async def main():
    global server
    runner = litmus.LSGIToASGIAdapter(app)
    server = litmus.Server(runner, listen_on="0.0.0.0:8000")
    server.ignite()
    await server.run_forever()


if __name__ == '__main__':
    asyncio.run(main())
