import asyncio
import litmus

from fastapi import FastAPI
litmus.set_log_level("info")
litmus.init_logger()

asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

app = FastAPI()
server = None


@app.get("/stats")
async def show_stats():
    print(server._server.len_clients())


async def main():
    global server
    runner = litmus.LSGIToASGIAdapter(app)
    server = litmus.Server(runner)
    server.ignite()
    await server.run_forever()


if __name__ == '__main__':
    asyncio.run(main())
