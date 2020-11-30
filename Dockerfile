FROM python:3.8

ENV PYTHONUNBUFFERED 1

RUN mkdir /temp
WORKDIR /temp
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN apt update
RUN apt install cmake -y

RUN mkdir /code
COPY . /code/
WORKDIR /code
RUN pip install maturin
RUN ls