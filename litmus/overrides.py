from typing import Optional


def init_logger(level: str, log_file: Optional[str], pretty: bool):  # noqa
    """
    Sets the internal server log level.

    Levels: [error, warning, info, debug, trace]

    trace levels log all internal server timings.
    """
    ...

