import typing as t

from importlib import import_module

from .endpoints import Blueprint

__all__ = ["Router"]


class Router:
    def __init__(self, web_app: object, app_files: t.Union[t.List[str], t.Tuple[str]], import_callback: t.Callable):
        self._app = web_app
        self._import_callback = import_callback

        setattr(self._app, 'add_blueprint', self.add_blueprint)

        self._apps = self._import_all(app_files)

    def _import_all(self, app_files: t.Union[t.List[str], t.Tuple[str]]) -> t.Dict[str, object]:
        apps: t.Dict[str, object] = {}
        for app_file in app_files:
            try:
                app = import_module(app_file)
            except ImportError:
                raise ImportError("Could not import module {} from current working directory".format(app_file))

            setup = getattr(app, "setup", None)
            if setup is not None:
                if isinstance(setup, t.Callable):
                    setup(self._app)
            apps[app_file] = app
        return apps

    def add_blueprint(self, bp: Blueprint):
        for ep in bp.endpoints:
            callback = getattr(bp, ep.callback_name, None)
            if callback is None:
                raise AttributeError("No endpoint called {} in blueprint {}", ep.callback_name, bp)
            ep.callback = callback
            self._import_callback(self._app, ep)
