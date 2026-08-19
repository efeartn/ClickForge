import json
import os
from platformdirs import user_config_dir

class Config:
    def __init__(self):
        self.config_dir = user_config_dir("ClickForge", "ClickForge")
        self.config_file = os.path.join(self.config_dir, "config.json")
        self._data = {}
        self.load()

    def get(self, key, default=None):
        return self._data.get(key, default)

    def set(self, key, value):
        self._data[key] = value

    def load(self):
        if os.path.exists(self.config_file):
            try:
                with open(self.config_file, "r") as f:
                    self._data = json.load(f)
            except Exception:
                self._data = {}

    def save(self):
        os.makedirs(self.config_dir, exist_ok=True)
        with open(self.config_file, "w") as f:
            json.dump(self._data, f)
