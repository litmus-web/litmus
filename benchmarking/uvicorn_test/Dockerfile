FROM python:3.8

WORKDIR /app

COPY ./requirements.txt .
COPY ./test.py .

RUN pip install -r requirements.txt

CMD ["python", "test.py"]