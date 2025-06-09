"""
Pokemon Shiny Bot - Main Environment
OpenAI Gym environment for Pokemon shiny hunting bot
"""

import numpy as np
import time

try:
    import gym
except ModuleNotFoundError:
    import gymnasium as gym

from config import (
    ACTION_NAMES, BUTTON_MAPPING, WINDOW_NAME, DISPLAY_SCALE_FACTOR,
    TOP_SCREEN_HEIGHT, IMAGE_WIDTH, MAX_CONSECUTIVE_FAILURES
)
from network import NetworkManager, shutdown_requested
from image_processing import gd2_to_rgb, split_screens, save_debug_frame, display_frame
from reward_system import PokemonRewardSystem
from loop_detection import LoopDetector


class PokemonEnvironment(gym.Env):
    """
    OpenAI Gym environment for Pokemon shiny hunting.
    
    Connects to a DeSmuME emulator via Lua script and provides
    a reinforcement learning interface for Pokemon gameplay.
    """
    
    def __init__(self):
        # Define observation and action spaces
        # Agent sees only the top screen: 256 × 192 × 3
        self.observation_space = gym.spaces.Box(
            0, 255, (TOP_SCREEN_HEIGHT, IMAGE_WIDTH, 3), np.uint8
        )
        self.action_space = gym.spaces.Discrete(len(ACTION_NAMES))
        self.action_names = ACTION_NAMES
        
        # Initialize components
        self.network_manager = NetworkManager()
        self.reward_system = PokemonRewardSystem()
        self.loop_detector = LoopDetector(ACTION_NAMES)
        
        # Frame tracking
        self._frame_counter = 0
        self._window_name = WINDOW_NAME
        
        # Connect to the game
        self._connect()
    
    def _connect(self):
        """Establish connection with the Lua script."""
        if not self.network_manager.wait_for_connection():
            raise ConnectionError("Failed to connect to bot")
    
    def step(self, action):
        """
        Execute one step in the environment.
        
        Args:
            action: Discrete action (0-12)
            
        Returns:
            tuple: (observation, reward, done, info)
        """
        if shutdown_requested:
            return np.zeros((TOP_SCREEN_HEIGHT, IMAGE_WIDTH, 3), dtype=np.uint8), 0.0, True, {"shutdown": True}
        
        try:
            # Receive frame from the game
            gd_blob = self.network_manager.receive_gd2_frame()
            
            # Process the image data
            full_rgb = gd2_to_rgb(gd_blob)
            top_screen, bottom_screen = split_screens(full_rgb)
            
            obs = top_screen  # Agent sees top screen
            pixels = full_rgb  # For display (both screens)
            
            # Convert action to button bytes
            action_bytes = self._action_to_bytes(action)
            
            # Send action to the game
            if not self.network_manager.send_action(action_bytes):
                raise ConnectionError("Failed to send action")
            
            # Calculate rewards
            reward = self._calculate_total_reward(obs, action)
            done = self._check_terminal(obs)
            
            # Handle debugging and display
            self._handle_frame_processing(obs, pixels, action, reward)
            
            self._frame_counter += 1
            return obs, reward, done, {}
            
        except (ConnectionError, OSError) as exc:
            return self._handle_connection_error(exc)
    
    def _action_to_bytes(self, action):
        """
        Convert discrete action to 12-byte button array.
        
        Args:
            action: Discrete action (0-12)
            
        Returns:
            bytes: 12-byte action array
        """
        action_bytes = [0] * 12  # All buttons off by default
        
        if action > 0:  # If not "Nothing"
            if action in BUTTON_MAPPING:
                button_index = BUTTON_MAPPING[action]
                action_bytes[button_index] = 1
        
        return bytes(action_bytes)
    
    def _calculate_total_reward(self, obs, action):
        """
        Calculate total reward from all reward components.
        
        Args:
            obs: Current observation
            action: Action taken
            
        Returns:
            float: Total reward
        """
        # Base reward from Pokemon gameplay
        reward = self.reward_system.compute_reward(obs)
        
        # Add loop detection penalties
        loop_penalty = self.loop_detector.detect_and_penalize_loops(obs, self._frame_counter)
        action_penalty = self.loop_detector.track_action(action, self._frame_counter)
        
        return reward + loop_penalty + action_penalty
    
    def _check_terminal(self, obs):
        """
        Check if the episode should terminate.
        
        Args:
            obs: Current observation
            
        Returns:
            bool: True if episode should end
        """
        # Check for global shutdown
        if shutdown_requested:
            return True
        
        # For now, never terminate (continuous exploration)
        return False
    
    def _handle_frame_processing(self, obs, pixels, action, reward):
        """
        Handle frame saving, display, and logging.
        
        Args:
            obs: Current observation
            pixels: Full screen pixels for display
            action: Action taken
            reward: Reward received
        """
        # Save first frame for debugging (one-time only)
        if self._frame_counter == 0:
            save_debug_frame(obs)
        
        # Display frames in real-time
        display_frame(pixels, self._window_name, DISPLAY_SCALE_FACTOR)
        
        # Log progress periodically
        if self._frame_counter % 500 == 0:
            self._log_progress(action, reward)
        
        # Periodic health report
        if self._frame_counter % 2000 == 0:
            self._log_health_report()
    
    def _log_progress(self, action, reward):
        """Log current progress."""
        action_name = (self.action_names[action] 
                      if action < len(self.action_names) 
                      else "Unknown")
        
        stats = self.network_manager.get_connection_stats()
        uptime = stats['uptime']
        
        print(f"[Environment] 🎮 Frame {self._frame_counter}: {action_name}, "
              f"reward: {reward:.3f}, uptime: {uptime:.1f}s")
    
    def _log_health_report(self):
        """Log detailed health report."""
        stats = self.network_manager.get_connection_stats()
        uptime = stats['uptime']
        last_comm = stats['last_communication']
        failures = stats['consecutive_failures']
        
        print(f"[Environment] 🔋 Health: Frame {self._frame_counter}, "
              f"{uptime:.1f}s uptime, {last_comm:.1f}s since last comm, "
              f"{failures} failures")
        
        # Also log reward system stats
        reward_stats = self.reward_system.get_stats()
        loop_stats = self.loop_detector.get_stats()
        
        print(f"[Environment] 📊 Stats: {reward_stats['visited_areas']} areas, "
              f"{loop_stats['consecutive_loops']} loops, "
              f"menu: {reward_stats['menu_state']}")
    
    def _handle_connection_error(self, exc):
        """
        Handle connection errors with reconnection logic.
        
        Args:
            exc: The exception that occurred
            
        Returns:
            tuple: (observation, reward, done, info)
        """
        print(f"[Environment] ❌ Connection error: {exc}")
        
        # Attempt to reconnect
        if self.network_manager.handle_connection_failure():
            # Return neutral state after reconnection
            return (np.zeros((TOP_SCREEN_HEIGHT, IMAGE_WIDTH, 3), dtype=np.uint8), 
                    0.0, False, {"reconnected": True})
        else:
            # Fatal error - can't reconnect
            print("[Environment] 💥 Fatal connection error")
            return (np.zeros((TOP_SCREEN_HEIGHT, IMAGE_WIDTH, 3), dtype=np.uint8), 
                    -1.0, True, {"fatal_error": True})
    
    def reset(self):
        """
        Reset the environment.
        
        Returns:
            numpy.ndarray: Initial observation
        """
        # Reset all subsystems
        self.reward_system.reset()
        self.loop_detector.reset()
        
        print("[Environment] 🔄 Resetting Pokemon environment...")
        
        # With the robust Lua script, we don't need special reset logic
        return np.zeros((TOP_SCREEN_HEIGHT, IMAGE_WIDTH, 3), dtype=np.uint8)
    
    def seed(self, seed=None):
        """
        Set random seed for reproducibility.
        
        Args:
            seed: Random seed
            
        Returns:
            list: List containing the seed
        """
        np.random.seed(seed)
        return [seed]
    
    def close(self):
        """Clean up resources."""
        print("[Environment] 🧹 Cleaning up environment...")
        self.network_manager.close()
        
        # Clean up display
        from image_processing import cleanup_display
        cleanup_display()
    
    def get_action_meanings(self):
        """Get human-readable action meanings."""
        return self.action_names
    
    def get_stats(self):
        """Get comprehensive environment statistics."""
        network_stats = self.network_manager.get_connection_stats()
        reward_stats = self.reward_system.get_stats()
        loop_stats = self.loop_detector.get_stats()
        
        return {
            'frame_counter': self._frame_counter,
            'network': network_stats,
            'rewards': reward_stats,
            'loops': loop_stats
        } 