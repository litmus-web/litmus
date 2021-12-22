import asyncio
import litmus as pyre

from fastapi import FastAPI
pyre.set_log_level("debug")
pyre.init_logger()

asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

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
    runner = pyre.LSGIToASGIAdapter(app)
    server = pyre.Server(runner, listen_on="0.0.0.0:8000")
    server.ignite()
    await server.run_forever()


if __name__ == '__main__':
    asyncio.run(main())
