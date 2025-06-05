import socket, numpy as np, torch, torch.nn as nn
from stable_baselines3 import PPO
env_sock = socket.socket()
env_sock.bind(('127.0.0.1', 5555))
env_sock.listen(1)
conn, _ = env_sock.accept()

class DSWrapper(gym.Env):
    def __init__(self):
        self.observation_space = gym.spaces.Box(0,255,(192,256,3),np.uint8)
        self.action_space      = gym.spaces.MultiBinary(12)
    def step(self, action):
        conn.send(bytes(action))
        pixels = conn.recv(256*192*3)
        obs = np.frombuffer(pixels,dtype=np.uint8).reshape(192,256,3)
        reward = compute_reward(obs)          # your shaping fn
        done   = check_terminal(obs)
        return obs, reward, done, {}
    def reset(self):
        return np.zeros((192,256,3),dtype=np.uint8)

env = DSWrapper()
model = PPO('CnnPolicy', env, verbose=1)
model.learn(total_timesteps=10_000)