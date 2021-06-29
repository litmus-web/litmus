import uvicorn
from fastapi import FastAPI


app = FastAPI()


@app.get("/hello")
async def show_stats():
    return "hello, world"


if __name__ == '__main__':
    uvicorn.run("uvicorn-framework-fastapi:app", log_level="warning", port=5000)
