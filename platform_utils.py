import os
import sys
import platform
from typing import Optional, List, Dict, Any

def get_platform() -> str:
    """
    Returns the current platform as a standardized string.
    Returns "windows", "macos", or "linux".
    """
    if sys.platform.startswith('win'):
        return "windows"
    elif sys.platform.startswith('darwin'):
        return "macos"
    elif sys.platform.startswith('linux'):
        return "linux"
    return sys.platform

def detect_display_server() -> str:
    """
    Detects the display server on Linux.
    Returns "x11", "wayland", or "unknown".
    """
    if get_platform() != "linux":
        return "unknown"
        
    # XDG_SESSION_TYPE is the most reliable way to check on modern Linux
    session_type = os.environ.get("XDG_SESSION_TYPE", "").lower()
    if session_type in ["wayland", "x11"]:
        return session_type
        
    # Fallbacks
    if "WAYLAND_DISPLAY" in os.environ:
        return "wayland"
    if "DISPLAY" in os.environ:
        return "x11"
        
    return "unknown"

def check_macos_accessibility() -> bool:
    """
    Checks if the application has macOS Accessibility permissions,
    which are required for simulating mouse clicks.
    Never crashes the app if pyobjc is missing.
    """
    if get_platform() != "macos":
        return True
        
    try:
        import ApplicationServices
        # Check if process is trusted, prompts user if not
        return ApplicationServices.AXIsProcessTrustedWithOptions({
            ApplicationServices.kAXTrustedCheckOptionPrompt: True
        })
    except ImportError:
        print("Warning: pyobjc-framework-ApplicationServices not installed. "
              "Cannot check macOS accessibility permissions.", file=sys.stderr)
        # We return True here so we don't block the user, even though
        # clicks might fail later.
        return True
    except Exception as e:
        print(f"Error checking macOS accessibility: {e}", file=sys.stderr)
        return True

def get_platform_warnings() -> List[Dict[str, str]]:
    """
    Returns a list of warnings specific to the current platform.
    """
    warnings = []
    current_platform = get_platform()
    
    if current_platform == "windows":
        pass # Windows usually works out of the box with pynput
        
    elif current_platform == "macos":
        if not check_macos_accessibility():
            warnings.append({
                "level": "error",
                "title": "Accessibility Permission Required",
                "message": "ClickForge needs Accessibility permissions to simulate mouse clicks on macOS.\n\n"
                           "Please grant permission in System Settings > Privacy & Security > Accessibility, "
                           "then restart the app."
            })
            
    elif current_platform == "linux":
        display_server = detect_display_server()
        if display_server == "wayland":
            warnings.append({
                "level": "warning",
                "title": "Wayland Limitations",
                "message": "You appear to be running Wayland. Synthetic mouse inputs "
                           "are restricted on Wayland for security reasons.\n\n"
                           "ClickForge may not work correctly or may only work within its own window. "
                           "Consider switching to an X11 session if you encounter issues."
            })
        elif display_server == "x11":
            warnings.append({
                "level": "info",
                "title": "X11 Supported",
                "message": "Running on X11. Full mouse simulation is supported."
            })
            
    return warnings

def show_platform_warnings(root: Any) -> None:
    """
    Shows platform warnings using message boxes.
    Called once at startup. Takes a CTk root window (or tkinter Tk).
    """
    warnings = get_platform_warnings()
    if not warnings:
        return
        
    try:
        import tkinter.messagebox as messagebox
        
        for warning in warnings:
            if warning["level"] == "error":
                messagebox.showerror(warning["title"], warning["message"], parent=root)
            elif warning["level"] == "warning":
                messagebox.showwarning(warning["title"], warning["message"], parent=root)
            elif warning["level"] == "info":
                pass # skip info to be less annoying
                
    except ImportError:
        print("tkinter not available for warnings.", file=sys.stderr)
