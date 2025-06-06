import socket, struct, numpy as np, zlib
# Gym / Gymnasium compatibility
try:
    import gym
except ModuleNotFoundError:
    import gymnasium as gym
try:
    import cv2                       # OpenCV for live display
except ModuleNotFoundError:
    cv2 = None
try:
    from stable_baselines3 import PPO
except ModuleNotFoundError:
    PPO = None

def gd2_to_rgb(gd_bytes):
    """
    Treat the incoming blob as raw BGRA data preceded by a GD-2 header.
    Extract the last 256*384*4 bytes as BGRA and convert to RGB.
    """
    raw_len = 256 * 384 * 4
    total_len = len(gd_bytes)
    if total_len < raw_len:
        raise ConnectionError(f"GD2 blob too short: {total_len} bytes (expected ≥ {raw_len})")

    # Assume header is at the start; take the last raw_len bytes
    raw = gd_bytes[-raw_len:]
    try:
        arr = np.frombuffer(raw, np.uint8).reshape(384, 256, 4)
    except ValueError as e:
        raise ConnectionError(f"Failed to reshape BGRA buffer: {e}")

    # Try different channel arrangements - maybe it's ARGB instead of BGRA
    # Let's try: channels 1,2,3 (skip alpha at position 0)
    return arr[..., [3, 2, 1]]  # Complete reverse order

LISTEN_PORT = 5555


# ------------------------------------------------------------------ #

def get_listener() -> socket.socket:
    """Return a listening TCP socket bound to 0.0.0.0:LISTEN_PORT."""
    s = socket.socket()
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("0.0.0.0", LISTEN_PORT))
    s.listen(1)
    return s

listener = get_listener()
print(f"[Server] Listening on 0.0.0.0:{LISTEN_PORT} …")

class DSWrapper(gym.Env):
    def __init__(self):
        # agent sees only the top panel: 256 × 192
        self.observation_space = gym.spaces.Box(0,255,(192,256,3),np.uint8)
        self.action_space      = gym.spaces.MultiBinary(12)
        self.conn = None
        self._frame_counter = 0
        self._window_name   = "DeSmuME Top‑Screen Feed"
        self._connect()

    # --------------------------------------------------
    # networking helpers
    # --------------------------------------------------
    def _connect(self):
        """Block until a bot connects, then store the socket."""
        self.conn, addr = listener.accept()
        print(f"[Server] Bot connected from {addr}")
        self.conn.settimeout(15)   # header + 147 456‑byte payload
        self._synchronize_connection()

    def _synchronize_connection(self):
        """Synchronize the connection by flushing any leftover data and sending initial action."""
        print("[Server] Synchronizing connection...")
        
        # Set a short timeout to flush any leftover data
        self.conn.settimeout(0.1)
        try:
            while True:
                leftover = self.conn.recv(1024)
                if not leftover:
                    break
                print(f"[Server] Flushed {len(leftover)} leftover bytes")
        except socket.timeout:
            pass  # This is expected when no more data
        
        # Reset to normal timeout
        self.conn.settimeout(15)
        
        # Send an initial action to kickstart the Lua script
        # All buttons released (12 zeros)
        initial_action = bytes([0] * 12)
        self.conn.sendall(initial_action)
        print("[Server] Sent initial action to synchronize")

    def _recv_exact(self, n):
        buf = bytearray()
        while len(buf) < n:
            remaining = n - len(buf)
            try:
                chunk = self.conn.recv(remaining)
            except socket.timeout:
                raise ConnectionError("Timed out mid-frame")
            if not chunk:
                raise ConnectionError("Bot closed connection mid‑frame")
            buf.extend(chunk)
        return bytes(buf)

    def step(self, action):
        try:
            # Read the "GD2" tag (3 bytes)
            tag3 = self._recv_exact(3)
            if tag3 != b"GD2":
                raise ConnectionError(f"Expected 'GD2' tag, got {tag3!r}")

            length = struct.unpack("<I", self._recv_exact(4))[0]
            if length < 20:
                raise ConnectionError(f"GD2 length too short: {length}")

            gd_blob = self._recv_exact(length)
            if len(gd_blob) < length:
                raise ConnectionError(f"Truncated GD2 blob ({len(gd_blob)} vs {length})")

            # Check for and flush any leftover data
            self.conn.settimeout(0.01)
            total_leftover = 0
            while True:
                try:
                    leftover = self.conn.recv(1024)
                    if leftover:
                        total_leftover += len(leftover)
                    else:
                        break
                except socket.timeout:
                    break
            self.conn.settimeout(15)

            # Decode the image data
            full_rgb = gd2_to_rgb(gd_blob)      # (384,256,3)
            
            # Debug: Check if top and bottom screens need different color handling
            top_screen = full_rgb[:192]         # First 192 rows (top screen)
            bottom_screen = full_rgb[192:]      # Last 192 rows (bottom screen)
            
            obs = top_screen                    # Agent sees top screen
            pixels = full_rgb                   # For display (both screens)

            # Send back the 12-byte action mask
            if hasattr(action, '__len__') and len(action) > 12:
                action_bytes = bytes([1 if x > 0.5 else 0 for x in action[:12]])
            else:
                action_list = list(action) if hasattr(action, '__len__') else [action]
                while len(action_list) < 12:
                    action_list.append(0)
                action_bytes = bytes([1 if x > 0.5 else 0 for x in action_list[:12]])
            
            self.conn.sendall(action_bytes)
            
            # Small delay for processing
            import time
            time.sleep(0.01)

        except (ConnectionError, OSError) as exc:
            print("[Server] connection lost:", exc)
            self.conn.close()
            self._connect()
            return np.zeros((192,256,3), dtype=np.uint8), 0.0, True, {}

        # ---- one‑time hex dump of first 16 bytes ----------------------
        if self._frame_counter == 0:
            # Check if top and bottom screens have different color patterns
            print("[DEBUG] Top screen first 16 bytes:", top_screen[:16].tobytes()[:48].hex(" "))
            print("[DEBUG] Bottom screen first 16 bytes:", bottom_screen[:16].tobytes()[:48].hex(" "))
        # ----------------------------------------------------------------

        # ─── debug + GUI every 100 frames ──────────────────────────────
        if self._frame_counter % 100 == 0:
            oh, ow = obs.shape[:2]
            print(f"[Server] frame {self._frame_counter}: w={ow} h={oh}")
            # Show image if cv2 is available
            if cv2 is not None:
                # Try displaying without color conversion first to see raw RGB
                full_display = pixels  # Full image (384x256x3) - both screens
                preview = cv2.resize(full_display, (0, 0), fx=2, fy=2, interpolation=cv2.INTER_LINEAR)  # type: ignore
                cv2.imshow(self._window_name, preview)  # type: ignore
                cv2.waitKey(1)  # type: ignore
        self._frame_counter += 1
        # ────────────────────────────────────────────────────────────────
        
        # 3) compute reward / terminal flags (stub)
        reward  = 0.0
        done    = False
        # ---------------- debug: dump first frame once -----------------
        if self._frame_counter == 0:
            print("[DBG] first 16 bytes:", pixels[:16].tobytes().hex(" "))
            try:
                import imageio.v3 as iio
                iio.imwrite("frame0.png", obs)   # pip install imageio[full]
                print("[DBG] Saved first frame as frame0.png")
            except ImportError:
                print("[DBG] imageio not available, skipping frame save")
        # ----------------------------------------------------------------
        return obs, reward, done, {}
    
    def reset(self):
        return np.zeros((192,256,3),dtype=np.uint8)

def compute_reward(obs): return 0.0
def check_terminal(obs): return False

# ------------------------------------------------------------------
# Train
# ------------------------------------------------------------------

env = DSWrapper()

# Add option to skip training for testing
SKIP_TRAINING = True  # Set to False if you want to train the AI

if SKIP_TRAINING:
    print("🎮 TESTING MODE - Skipping AI training, just displaying frames...")
    print("Press Ctrl+C to stop")
    
    # Simple test loop to keep receiving frames
    try:
        frame_count = 0
        while True:
            # Send random actions (all buttons off for now)
            action = [0] * 12  # No buttons pressed
            obs, reward, done, info = env.step(action)
            frame_count += 1
            
            if frame_count % 500 == 0:  # Less frequent logging
                print(f"[Test Mode] Received {frame_count} frames successfully")
                
    except KeyboardInterrupt:
        print("\n[Test Mode] Stopped by user (Ctrl+C)")
    except Exception as e:
        print(f"[Test Mode] Error: {e}")
        
elif PPO is None:
    print("⚠️  stable_baselines3 is not installed.")
    print("Running in test mode - will display received frames...")
    
    # Simple test loop to keep receiving frames
    try:
        frame_count = 0
        while True:
            # Send random actions (all buttons off for now)
            action = [0] * 12  # No buttons pressed
            obs, reward, done, info = env.step(action)
            frame_count += 1
            
            if frame_count % 100 == 0:
                print(f"[Test Mode] Received {frame_count} frames successfully")
                
    except KeyboardInterrupt:
        print("\n[Test Mode] Stopped by user (Ctrl+C)")
    except Exception as e:
        print(f"[Test Mode] Error: {e}")
else:
    print("🤖 TRAINING MODE - Starting AI training...")
    model = PPO("CnnPolicy", env, verbose=1)
    model.learn(total_timesteps=10_000)
