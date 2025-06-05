import socket, argparse, numpy as np, torch, torch.nn as nn
from stable_baselines3 import PPO
import gym   # you already import it elsewhere

# ------------ NEW: CLI flags -------------
parser = argparse.ArgumentParser()
parser.add_argument("--host", default="0.0.0.0")
parser.add_argument("--port", type=int, default=5555)
args = parser.parse_args()
# -----------------------------------------

env_sock = socket.socket()
env_sock.bind((args.host, args.port))
env_sock.listen(1)
print(f"[Server] Waiting on {args.host}:{args.port} …")
conn, addr = env_sock.accept()
print(f"[Server] Bot connected from {addr}")

class DSWrapper(gym.Env):
    def __init__(self):
        self.observation_space = gym.spaces.Box(0,255,(192,256,3),np.uint8)
        self.action_space      = gym.spaces.MultiBinary(12)
    def step(self, action):
        conn.send(bytes(action))
        pixels = conn.recv(256*192*3)
        obs = np.frombuffer(pixels,dtype=np.uint8).reshape(192,256,3)
        reward = compute_reward(obs)
        done   = check_terminal(obs)
        return obs, reward, done, {}
    def reset(self):
        return np.zeros((192,256,3),dtype=np.uint8)

env   = DSWrapper()
model = PPO('CnnPolicy', env, verbose=1)
model.learn(total_timesteps=10_000)