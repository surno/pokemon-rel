import socket, struct, gym, numpy as np
from stable_baselines3 import PPO

LISTEN_PORT = 5555

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
        self.observation_space = gym.spaces.Box(0,255,(192,256,3),np.uint8)
        self.action_space      = gym.spaces.MultiBinary(12)
        self.conn = None
        self._frame_counter = 0
        self._connect()

    # --------------------------------------------------
    # networking helpers
    # --------------------------------------------------
    def _connect(self):
        """Block until a bot connects, then store the socket."""
        self.conn, addr = listener.accept()
        print(f"[Server] Bot connected from {addr}")
        self.conn.settimeout(15)   # header + 147 456‑byte payload

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
            hdr = self._recv_exact(8)
            w, h = struct.unpack("<II", hdr)
            # sanity‑check: DeSmuME top screen must be 256×192
            if w != 256 or h != 192:
                print(f"[Server] WARNING: received header {w}×{h}; skipping frame")
                # read and discard whatever payload arrives this frame
                discard = self._recv_exact(min(w * h * 3, 512 * 1024))
                return np.zeros((192,256,3),dtype=np.uint8), 0.0, False, {}

            pixels = self._recv_exact(256 * 192 * 3)
            #  send back the 12‑byte action mask (all‑zeros placeholder)
            self.conn.sendall(bytes(action))
        except (ConnectionError, OSError) as exc:
            print("[Server] connection lost:", exc)
            self.conn.close()
            self._connect()
            # return zero observation, done=True so PPO resets episode
            return np.zeros((192,256,3),dtype=np.uint8), 0.0, True, {}

        obs = np.frombuffer(pixels, dtype=np.uint8).reshape(h, w, 3)

        # ─── debug: print every 100th frame size ───────────────────────
        if self._frame_counter % 100 == 0:
            print(f"[Server] frame {self._frame_counter}: "
                  f"w={w} h={h}  bytes={len(pixels)}")
        self._frame_counter += 1
        # ────────────────────────────────────────────────────────────────
        
        # 3) compute reward / terminal flags (stub)
        reward  = 0.0
        done    = False
        return obs, reward, done, {}
    
    def reset(self):
        return np.zeros((192,256,3),dtype=np.uint8)

def compute_reward(obs): return 0.0
def check_terminal(obs): return False

# ------------------------------------------------------------------
# Train
# ------------------------------------------------------------------

env = DSWrapper()
model = PPO("CnnPolicy", env, verbose=1)
model.learn(total_timesteps=10_000)