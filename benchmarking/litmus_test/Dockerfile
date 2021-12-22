FROM python:3.8

ENV PYTHONUNBUFFERED 1

RUN mkdir /temp
WORKDIR /temp
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN mkdir /code
WORKDIR /code
RUN git clone https://github.com/litmus-web/litmus.git

WORKDIR /code/litmus
COPY ./requirements.txt .
RUN python -m venv /opt/venv
RUN . /opt/venv/bin/activate && pip install -r requirements.txt
RUN . /opt/venv/bin/activate && maturin develop --release

COPY ./test.py ./test.py


CMD . /opt/venv/bin/activate && exec python test.py