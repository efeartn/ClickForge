import sys

def get_platform() -> str:
    if sys.platform.startswith('win'): return 'windows'
    elif sys.platform.startswith('darwin'): return 'macos'
    else: return 'linux'

def show_platform_warnings(root):
    pass
