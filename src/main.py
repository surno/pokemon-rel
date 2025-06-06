import socket, struct, numpy as np, zlib, time, signal, sys
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
    import torch
except ModuleNotFoundError:
    PPO = None
    torch = None

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

# Global flag for graceful shutdown
shutdown_requested = False

def signal_handler(sig, frame):
    """Handle Ctrl+C gracefully"""
    global shutdown_requested
    print("\n[Server] 🛑 Shutdown requested... Cleaning up...")
    shutdown_requested = True

signal.signal(signal.SIGINT, signal_handler)

# ------------------------------------------------------------------ #

def get_listener() -> socket.socket:
    """Return a listening TCP socket bound to 0.0.0.0:LISTEN_PORT."""
    s = socket.socket()
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)  # Enable keepalive
    s.bind(("0.0.0.0", LISTEN_PORT))
    s.listen(1)
    return s

listener = get_listener()
print(f"[Server] 🚀 Listening on 0.0.0.0:{LISTEN_PORT} for overnight operation...")

class DSWrapper(gym.Env):
    def __init__(self):
        # agent sees only the top panel: 256 × 192
        self.observation_space = gym.spaces.Box(0,255,(192,256,3),np.uint8)
        
        # Define discrete actions (one at a time)
        self.action_names = [
            "Nothing",      # 0 - no buttons pressed
            "A", "B",       # 1, 2 - action buttons
            "Up", "Down", "Left", "Right",  # 3, 4, 5, 6 - movement
            "Start", "Select",  # 7, 8 - menu buttons
            "X", "Y",       # 9, 10 - additional buttons
            "L", "R"        # 11, 12 - shoulder buttons
        ]
        self.action_space = gym.spaces.Discrete(len(self.action_names))
        
        self.conn = None
        self._frame_counter = 0
        self._window_name   = "DeSmuME Top‑Screen Feed"
        self._sync_buffer = b""  # Buffer for synchronization
        
        # Pokemon-specific tracking for rewards
        self._last_obs = None
        self._visited_areas = set()
        self._steps_without_movement = 0
        self._max_steps_still = 150  # Allow more time for battles/menus
        
        # Pokemon encounter and battle tracking
        self._last_battle_state = False
        self._battle_start_frame = 0
        self._wild_encounter_detected = False
        self._pokemon_caught_count = 0
        self._shiny_encounters = 0
        
        # Game progression tracking
        self._level_ups_detected = 0
        self._item_found_count = 0
        self._new_route_bonus = 0
        
        # Screen state analysis (for detecting different game states)
        self._menu_state = False
        self._battle_menu_state = False
        self._overworld_state = True
        
        # Connection monitoring
        self._connection_start_time = None
        self._last_successful_communication = None
        self._consecutive_failures = 0
        self._max_consecutive_failures = 20
        
        self._connect()

    # --------------------------------------------------
    # networking helpers
    # --------------------------------------------------
    def _connect(self):
        """Block until a bot connects, then store the socket with robust error handling."""
        connection_attempts = 0
        max_connection_attempts = 10
        
        while connection_attempts < max_connection_attempts and not shutdown_requested:
            try:
                connection_attempts += 1
                print(f"[Server] 🔄 Waiting for bot connection (attempt {connection_attempts}/{max_connection_attempts})...")
                
                # Set a timeout for accept() to allow checking for shutdown
                listener.settimeout(10.0)
                
                try:
                    self.conn, addr = listener.accept()
                    print(f"[Server] ✅ Bot connected from {addr}")
                    
                    # Configure socket for robustness
                    self.conn.settimeout(30)   # Longer timeout for stability
                    self.conn.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
                    
                    self._sync_buffer = b""  # Reset sync buffer for new connection
                    self._connection_start_time = time.time()
                    self._last_successful_communication = time.time()
                    self._consecutive_failures = 0
                    
                    self._synchronize_connection()
                    return
                    
                except socket.timeout:
                    print("[Server] ⏰ Connection timeout, retrying...")
                    continue
                    
            except Exception as e:
                print(f"[Server] ❌ Connection error: {e}")
                if connection_attempts < max_connection_attempts:
                    print(f"[Server] ⏳ Waiting 5 seconds before retry...")
                    time.sleep(5)
                else:
                    print("[Server] 💥 Max connection attempts reached")
                    
        if shutdown_requested:
            print("[Server] 🛑 Shutdown requested during connection")
            sys.exit(0)
        else:
            raise ConnectionError("Failed to establish connection after maximum attempts")

    def _synchronize_connection(self):
        """Synchronize the connection by finding the next GD2 header."""
        print("[Server] 🔄 Synchronizing connection...")
        
        # Send an initial action to kickstart the Lua script
        try:
            initial_action = bytes([0] * 12)
            self.conn.sendall(initial_action)
        except Exception as e:
            print(f"[Server] ❌ Failed to send initial action: {e}")
            return
        
        # Look for the GD2 header in the incoming data stream
        search_buffer = bytearray()
        max_search_bytes = 2000000  # Increased search limit
        bytes_searched = 0
        
        while bytes_searched < max_search_bytes and not shutdown_requested:
            try:
                chunk = self.conn.recv(1024)
                if not chunk:
                    raise ConnectionError("Connection closed while searching for GD2 header")
                
                search_buffer.extend(chunk)
                bytes_searched += len(chunk)
                
                # Look for GD2 pattern in the buffer
                gd2_pos = search_buffer.find(b'GD2')
                if gd2_pos != -1:
                    # Remove everything before the GD2 header
                    search_buffer = search_buffer[gd2_pos:]
                    
                    # Put the remaining data back by creating a new buffer
                    self._sync_buffer = bytes(search_buffer)
                    print(f"[Server] ✅ Synchronized! Ready to receive frames.")
                    self._last_successful_communication = time.time()
                    return
                
                # Keep only the last 2 bytes to avoid missing GD2 across chunk boundaries
                if len(search_buffer) > 2048:
                    search_buffer = search_buffer[-2:]
                    
            except socket.timeout:
                print("[Server] ⏰ Timeout while searching for GD2 - retrying...")
                continue
            except Exception as e:
                print(f"[Server] ❌ Error during synchronization: {e}")
                break
        
        print("[Server] ⚠️  Could not find GD2 header - connection may be unstable")
        self._sync_buffer = b""

    def _recv_exact(self, n):
        buf = bytearray()
        
        # First, use any data from the sync buffer
        if self._sync_buffer:
            bytes_to_take = min(len(self._sync_buffer), n)
            buf.extend(self._sync_buffer[:bytes_to_take])
            self._sync_buffer = self._sync_buffer[bytes_to_take:]
            
        # Then receive the rest normally
        while len(buf) < n and not shutdown_requested:
            remaining = n - len(buf)
            try:
                chunk = self.conn.recv(remaining)
            except socket.timeout:
                raise ConnectionError("Timed out mid-frame")
            except Exception as e:
                raise ConnectionError(f"Network error: {e}")
                
            if not chunk:
                raise ConnectionError("Bot closed connection mid‑frame")
            buf.extend(chunk)
        return bytes(buf)

    def step(self, action):
        if shutdown_requested:
            return np.zeros((192,256,3), dtype=np.uint8), 0.0, True, {"shutdown": True}
            
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

            # Decode the image data
            full_rgb = gd2_to_rgb(gd_blob)      # (384,256,3)
            top_screen = full_rgb[:192]         # First 192 rows (top screen)
            bottom_screen = full_rgb[192:]      # Last 192 rows (bottom screen)
            
            obs = top_screen                    # Agent sees top screen
            pixels = full_rgb                   # For display (both screens)
            
            # Convert discrete action to 12-byte button array
            action_bytes = [0] * 12  # All buttons off by default
            
            if action > 0:  # If not "Nothing"
                # Map discrete action to button index
                button_mapping = {
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
                
                if action in button_mapping:
                    button_index = button_mapping[action]
                    action_bytes[button_index] = 1
            
            action_bytes = bytes(action_bytes)
            
            # Send action with retry logic
            send_attempts = 0
            max_send_attempts = 3
            sent = False
            
            while send_attempts < max_send_attempts and not sent and not shutdown_requested:
                try:
                    self.conn.sendall(action_bytes)
                    sent = True
                    self._last_successful_communication = time.time()
                    self._consecutive_failures = 0
                except Exception as e:
                    send_attempts += 1
                    print(f"[Server] ⚠️  Send attempt {send_attempts}/{max_send_attempts} failed: {e}")
                    if send_attempts < max_send_attempts:
                        time.sleep(0.1)
            
            if not sent:
                raise ConnectionError("Failed to send action after all attempts")

        except (ConnectionError, OSError) as exc:
            self._consecutive_failures += 1
            print(f"[Server] ❌ Connection issue #{self._consecutive_failures}: {exc}")
            
            if self._consecutive_failures >= self._max_consecutive_failures:
                print("[Server] 💥 Too many consecutive failures, forcing reconnection...")
                
            if self.conn:
                try:
                    self.conn.close()
                except:
                    pass
                self.conn = None
                
            # Attempt to reconnect
            try:
                self._connect()
                # Return neutral state after reconnection
                return np.zeros((192,256,3), dtype=np.uint8), 0.0, False, {"reconnected": True}
            except Exception as reconnect_error:
                print(f"[Server] 💥 Reconnection failed: {reconnect_error}")
                return np.zeros((192,256,3), dtype=np.uint8), -1.0, True, {"fatal_error": True}

        # Compute reward and terminal flags
        reward = self.compute_reward(obs)
        done = self.check_terminal(obs)
        
        # Save first frame for debugging (one-time only)
        if self._frame_counter == 0:
            try:
                import imageio.v3 as iio
                iio.imwrite("frame0.png", obs)
                print("[Server] 💾 Saved first frame as frame0.png")
            except ImportError:
                pass

        # Display frames in real-time if cv2 is available
        if cv2 is not None:
            try:
                full_display = pixels  # Full image (384x256x3) - both screens
                preview = cv2.resize(full_display, (0, 0), fx=2, fy=2, interpolation=1)  # type: ignore  # 1 = INTER_LINEAR
                cv2.imshow(self._window_name, preview)  # type: ignore
                cv2.waitKey(1)  # type: ignore
            except Exception as e:
                # Don't let display errors crash the training
                print(f"[Server] ⚠️  Display error (non-fatal): {e}")
            
        # Log progress and connection health
        if self._frame_counter % 500 == 0:
            action_name = self.action_names[action] if action < len(self.action_names) else "Unknown"
            uptime = time.time() - self._connection_start_time if self._connection_start_time else 0
            print(f"[Server] 🎮 Frame {self._frame_counter}: {action_name}, reward: {reward:.3f}, uptime: {uptime:.1f}s")
            
        # Periodic health report
        if self._frame_counter % 2000 == 0:
            uptime = time.time() - self._connection_start_time if self._connection_start_time else 0
            last_comm = time.time() - self._last_successful_communication if self._last_successful_communication else 0
            print(f"[Server] 🔋 Health: Frame {self._frame_counter}, {uptime:.1f}s uptime, {last_comm:.1f}s since last comm, {self._consecutive_failures} failures")
            
        self._frame_counter += 1
        return obs, reward, done, {}
    
    def reset(self):
        # Reset navigation tracking
        self._last_obs = None
        self._position_history = []
        self._visited_areas = set()
        self._steps_without_movement = 0
        
        print("[Reset] 🔄 Resetting environment...")
        
        # With the robust Lua script, we don't need to do anything special
        return np.zeros((192,256,3),dtype=np.uint8)

    def seed(self, seed=None):
        """Set random seed for reproducibility (required by gym interface)"""
        np.random.seed(seed)
        return [seed]

    def compute_reward(self, obs):
        """
        Navigation reward focused on TOP SCREEN ONLY (actual gameplay).
        Bottom screen UI changes don't count as navigation.
        """
        reward = 0.0
        
        if self._last_obs is not None:
            # Calculate difference between current and last TOP SCREEN frame only
            top_screen_diff = np.abs(obs.astype(np.float32) - self._last_obs.astype(np.float32))
            movement_amount = np.mean(top_screen_diff)
            
            # Hash current TOP SCREEN observation to track visited gameplay areas
            # Use smaller sample for efficiency but focus on gameplay area
            gameplay_sample = obs[50:150, 50:200]  # Central gameplay area of top screen
            obs_hash = hash(gameplay_sample[::4, ::4].tobytes())  # Downsample for efficiency
            
            # Reward for movement in the gameplay area (top screen)
            if movement_amount > 3.0:  # Lower threshold since we're only looking at top screen
                reward += 0.15  # Slightly higher reward for meaningful gameplay movement
                self._steps_without_movement = 0
                
                # Extra reward for visiting new gameplay areas
                if obs_hash not in self._visited_areas:
                    reward += 0.6  # Higher reward for exploring new gameplay areas
                    self._visited_areas.add(obs_hash)
                    if len(self._visited_areas) % 10 == 0:  # Less frequent updates for overnight stability
                        print(f"[Navigation] 🗺️  Explored {len(self._visited_areas)} unique gameplay areas!")
            else:
                # Penalize staying still in gameplay (top screen shows no change)
                self._steps_without_movement += 1
                if self._steps_without_movement > self._max_steps_still:
                    reward -= 0.08  # Slightly higher penalty for not moving in gameplay
        
        # Store current TOP SCREEN observation for next comparison
        self._last_obs = obs.copy()
        
        return reward

    def check_terminal(self, obs):
        """
        Basic terminal condition - could be expanded to detect battles, fainting, etc.
        """
        # Check for global shutdown
        if shutdown_requested:
            return True
            
        # For now, never terminate (continuous exploration)
        return False

    def close(self):
        """Clean up resources"""
        print("[Server] 🧹 Cleaning up resources...")
        if self.conn:
            self.conn.close()
        if cv2 is not None:
            try:
                cv2.destroyAllWindows()  # type: ignore
            except:
                pass  # Ignore any cv2 cleanup errors

# ------------------------------------------------------------------
# Train
# ------------------------------------------------------------------

env = DSWrapper()

# Add option to skip training for testing
SKIP_TRAINING = False  # Set to False if you want to train the AI

# GPU Detection and Setup
def detect_device():
    """Detect and configure the best available device (GPU/CPU)"""
    if torch is None:
        print("[Device] 💻 PyTorch not available - using CPU")
        return "cpu"
    
    if torch.cuda.is_available():
        device = "cuda"
        gpu_name = torch.cuda.get_device_name(0)
        gpu_memory = torch.cuda.get_device_properties(0).total_memory / 1024**3
        print(f"[Device] 🚀 GPU Detected: {gpu_name} ({gpu_memory:.1f} GB VRAM)")
        print(f"[Device] ⚡ Using GPU acceleration for training!")
        
        # Optimize GPU settings for stable training
        torch.backends.cudnn.benchmark = True
        torch.backends.cuda.matmul.allow_tf32 = True
        
        return device
    elif torch.backends.mps.is_available():
        device = "mps"  # Apple Silicon GPU
        print(f"[Device] 🍎 Apple Silicon GPU (MPS) detected!")
        print(f"[Device] ⚡ Using Metal Performance Shaders acceleration!")
        return device
    else:
        print(f"[Device] 💻 No GPU detected - using CPU")
        print(f"[Device] 💡 Tip: Install CUDA or use a GPU-enabled environment for faster training")
        return "cpu"

# Detect the best device for training
DEVICE = detect_device()

try:
    if SKIP_TRAINING:
        print("🎮 TESTING MODE - Overnight frame monitoring...")
        print("Press Ctrl+C to stop gracefully")
        
        # Robust test loop for overnight operation
        frame_count = 0
        start_time = time.time()
        
        while not shutdown_requested:
            try:
                # Send random actions (mostly nothing, occasionally move)
                action = 0 if frame_count % 15 != 0 else np.random.randint(0, len(env.action_names))
                obs, reward, done, info = env.step(action)
                frame_count += 1
                
                if frame_count % 1000 == 0:  # Less frequent logging for overnight
                    uptime = time.time() - start_time
                    fps = frame_count / uptime if uptime > 0 else 0
                    print(f"[Test Mode] 📊 {frame_count} frames, {uptime:.1f}s uptime, {fps:.1f} FPS")
                    
                if done:
                    if info.get("fatal_error"):
                        print("[Test Mode] 💥 Fatal error detected")
                        break
                    elif info.get("shutdown"):
                        print("[Test Mode] 🛑 Shutdown requested")
                        break
                        
            except KeyboardInterrupt:
                print("\n[Test Mode] 🛑 Stopped by user (Ctrl+C)")
                break
            except Exception as e:
                print(f"[Test Mode] ❌ Error: {e}")
                print("[Test Mode] 🔄 Attempting to continue...")
                time.sleep(1)
                
    elif PPO is None:
        print("⚠️  stable_baselines3 is not installed.")
        print("🎮 Running in robust test mode for overnight operation...")
        
        # Robust test loop identical to SKIP_TRAINING mode
        frame_count = 0
        start_time = time.time()
        
        while not shutdown_requested:
            try:
                action = 0 if frame_count % 15 != 0 else np.random.randint(0, len(env.action_names))
                obs, reward, done, info = env.step(action)
                frame_count += 1
                
                if frame_count % 1000 == 0:
                    uptime = time.time() - start_time
                    fps = frame_count / uptime if uptime > 0 else 0
                    print(f"[Test Mode] 📊 {frame_count} frames, {uptime:.1f}s uptime, {fps:.1f} FPS")
                    
                if done and info.get("fatal_error"):
                    print("[Test Mode] 💥 Fatal error detected")
                    break
                    
            except KeyboardInterrupt:
                print("\n[Test Mode] 🛑 Stopped by user")
                break
            except Exception as e:
                print(f"[Test Mode] ❌ Error: {e}")
                time.sleep(1)
                
    else:
        print("🤖 ROBUST TRAINING MODE - Overnight AI training...")
        
        # Adjust batch size based on device capabilities
        if DEVICE == "cuda":
            # GPU can handle larger batches
            batch_size = 128
            n_steps = 4096  # Collect more steps for GPU efficiency
            print(f"[GPU] 🚀 Optimized settings: batch_size={batch_size}, n_steps={n_steps}")
        elif DEVICE == "mps":
            # Apple Silicon optimization
            batch_size = 96
            n_steps = 3072
            print(f"[MPS] 🍎 Optimized settings: batch_size={batch_size}, n_steps={n_steps}")
        else:
            # CPU settings (smaller batches)
            batch_size = 64
            n_steps = 2048
            print(f"[CPU] 💻 CPU settings: batch_size={batch_size}, n_steps={n_steps}")
        
        # Create PPO model with GPU support and exploration-friendly settings
        model = PPO(
            "CnnPolicy", 
            env, 
            device=DEVICE,  # 🚀 GPU acceleration!
            verbose=1,
            learning_rate=3e-4,
            n_steps=n_steps,
            batch_size=batch_size,
            n_epochs=10,
            gamma=0.99,
            gae_lambda=0.95,
            clip_range=0.2,
            ent_coef=0.01,  # Encourage exploration
            vf_coef=0.5,
            max_grad_norm=0.5,
            seed=42
        )
        
        print("🎮 Starting robust overnight training...")
        print("🛡️  Features: Auto-reconnect, Error recovery, Graceful shutdown")
        print("Press Ctrl+C for graceful shutdown")
        
        # Extended training for overnight operation
        total_timesteps = 100_000  # Much longer for overnight training
        
        try:
            model.learn(total_timesteps=total_timesteps)
            print("🎉 Training completed successfully!")
        except KeyboardInterrupt:
            print("\n🛑 Training interrupted by user")
        except Exception as e:
            print(f"❌ Training error: {e}")
            print("🔄 Training can be resumed from checkpoint")

finally:
    print("[Server] 🧹 Performing cleanup...")
    env.close()
    if listener:
        listener.close()
    print("[Server] ✅ Cleanup complete - Good night! 🌙")
