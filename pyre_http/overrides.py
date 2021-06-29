

def set_log_level(level: str):  # noqa
    """
    Sets the internal server log level.

    Levels: [error, warning, info, debug, trace]

    trace levels log all internal server timings.
    """
    ...


def init_logger():
    """
    Initialises the logger for the server, this can only be called once.

    No logs will be displayed without calling this first however once called
    the level can no-longer be adjusted.
    """
    ...

