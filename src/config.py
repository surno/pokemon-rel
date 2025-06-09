"""
Configuration settings for Pokemon Shiny Bot
"""

# Network Configuration
LISTEN_PORT = 5555

# Image Processing Constants
IMAGE_WIDTH = 256
IMAGE_HEIGHT = 384
TOP_SCREEN_HEIGHT = 192
BYTES_PER_PIXEL = 4
RAW_IMAGE_SIZE = IMAGE_WIDTH * IMAGE_HEIGHT * BYTES_PER_PIXEL

# Pokemon Game Action Mapping
ACTION_NAMES = [
    "Nothing",      # 0 - no buttons pressed
    "A", "B",       # 1, 2 - action buttons
    "Up", "Down", "Left", "Right",  # 3, 4, 5, 6 - movement
    "Start", "Select",  # 7, 8 - menu buttons
    "X", "Y",       # 9, 10 - additional buttons
    "L", "R"        # 11, 12 - shoulder buttons
]

BUTTON_MAPPING = {
    1: 0,   # A
    2: 1,   # B
    3: 4,   # Up
    4: 5,   # Down
    5: 6,   # Left
    6: 7,   # Right
    7: 3,   # Start
    8: 2,   # Select
    9: 8,   # X
    10: 9,  # Y
    11: 10, # L
    12: 11  # R
}

# Reward System Configuration
MAX_STEPS_STILL = 150
LOOP_DETECTION_WINDOW = 8
FRAME_HISTORY_SIZE = 20
ACTION_HISTORY_SIZE = 15
ACTION_LOOP_THRESHOLD = 4
MAX_CONSECUTIVE_LOOPS = 3

# Connection Configuration
MAX_CONNECTION_ATTEMPTS = 10
MAX_CONSECUTIVE_FAILURES = 20
MAX_SEND_ATTEMPTS = 3
CONNECTION_TIMEOUT = 30
SYNC_TIMEOUT = 10

# Training Configuration
DEFAULT_TOTAL_TIMESTEPS = 100_000
GPU_BATCH_SIZE = 128
GPU_N_STEPS = 4096
MPS_BATCH_SIZE = 96
MPS_N_STEPS = 3072
CPU_BATCH_SIZE = 64
CPU_N_STEPS = 2048

# Display Configuration
DISPLAY_SCALE_FACTOR = 2
WINDOW_NAME = "DeSmuME Top‑Screen Feed" 