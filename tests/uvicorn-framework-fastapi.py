import uvicorn
from fastapi import FastAPI


app = FastAPI()


@app.get("/stats")
async def show_stats():
    return "litmus.statistics()"


if __name__ == '__main__':
    uvicorn.run("uvicorn-framework-fastapi:app", log_level="warning", port=5000)
