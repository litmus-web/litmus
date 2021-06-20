import asyncio
import uvicorn

from fastapi import FastAPI

app = FastAPI()


@app.get("/hello/{name}")
async def hello(name: str):
    return f"Hello, {name}!"


uvicorn.run(app, port=8000, log_level="critical")
