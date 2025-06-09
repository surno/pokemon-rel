"""
Image processing utilities for Pokemon Shiny Bot
Handles conversion of game screen data to usable formats
"""

import numpy as np
from config import RAW_IMAGE_SIZE, IMAGE_HEIGHT, IMAGE_WIDTH, BYTES_PER_PIXEL

try:
    import cv2
except ModuleNotFoundError:
    cv2 = None

try:
    import imageio.v3 as iio
except ModuleNotFoundError:
    iio = None


def gd2_to_rgb(gd_bytes):
    """
    Treat the incoming blob as raw BGRA data preceded by a GD-2 header.
    Extract the last 256*384*4 bytes as BGRA and convert to RGB.
    
    Args:
        gd_bytes: Raw bytes from the game containing GD2 header and image data
        
    Returns:
        numpy.ndarray: RGB image array of shape (384, 256, 3)
        
    Raises:
        ConnectionError: If the data is too short or malformed
    """
    raw_len = RAW_IMAGE_SIZE
    total_len = len(gd_bytes)
    
    if total_len < raw_len:
        raise ConnectionError(f"GD2 blob too short: {total_len} bytes (expected ≥ {raw_len})")

    # Assume header is at the start; take the last raw_len bytes
    raw = gd_bytes[-raw_len:]
    
    try:
        arr = np.frombuffer(raw, np.uint8).reshape(IMAGE_HEIGHT, IMAGE_WIDTH, BYTES_PER_PIXEL)
    except ValueError as e:
        raise ConnectionError(f"Failed to reshape BGRA buffer: {e}")

    # Convert BGRA to RGB (complete reverse order)
    return arr[..., [3, 2, 1]]


def split_screens(full_rgb):
    """
    Split the full DS screen into top and bottom screens.
    
    Args:
        full_rgb: Full RGB image array of shape (384, 256, 3)
        
    Returns:
        tuple: (top_screen, bottom_screen) where each is shape (192, 256, 3)
    """
    top_screen = full_rgb[:192]   # First 192 rows (top screen)
    bottom_screen = full_rgb[192:]  # Last 192 rows (bottom screen)
    return top_screen, bottom_screen


def save_debug_frame(image, filename="frame0.png"):
    """
    Save a frame for debugging purposes.
    
    Args:
        image: Image array to save
        filename: Output filename
    """
    if iio is not None:
        try:
            iio.imwrite(filename, image)
            print(f"[Debug] 💾 Saved frame as {filename}")
        except Exception as e:
            print(f"[Debug] ❌ Failed to save frame: {e}")
    else:
        print("[Debug] ⚠️  imageio not available for frame saving")


def display_frame(pixels, window_name, scale_factor=2):
    """
    Display frame in real-time using OpenCV if available.
    
    Args:
        pixels: Image array to display
        window_name: Name of the display window
        scale_factor: How much to scale the display
    """
    if cv2 is not None:
        try:
            # Scale up for better visibility
            preview = cv2.resize(  # type: ignore
                pixels, 
                (0, 0), 
                fx=scale_factor, 
                fy=scale_factor, 
                interpolation=cv2.INTER_LINEAR  # type: ignore
            )
            cv2.imshow(window_name, preview)  # type: ignore
            cv2.waitKey(1)  # type: ignore
        except Exception as e:
            # Don't let display errors crash the training
            print(f"[Display] ⚠️  Display error (non-fatal): {e}")
    else:
        # Silently skip if cv2 not available
        pass


def cleanup_display():
    """Clean up OpenCV windows."""
    if cv2 is not None:
        try:
            cv2.destroyAllWindows()  # type: ignore
        except:
            pass  # Ignore any cleanup errors 