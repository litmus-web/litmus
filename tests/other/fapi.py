import uvicorn
from fastapi import FastAPI, Request


REMOVE_PATH = "/service/api"
app = FastAPI()


@app.middleware("http")
async def alter_path(request: Request, call_next):
    current: str = request.scope['path']
    request.scope['path'] = current.replace(REMOVE_PATH, "", 1)
    return await call_next(request)


@app.get("/foo")
async def yes():
    return "hello"


if __name__ == '__main__':
    uvicorn.run("fapi:app")