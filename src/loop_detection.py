"""
Loop detection and prevention system for Pokemon Shiny Bot
Prevents the AI from getting stuck in repetitive patterns
"""

import numpy as np
from config import (
    LOOP_DETECTION_WINDOW, FRAME_HISTORY_SIZE, ACTION_HISTORY_SIZE,
    ACTION_LOOP_THRESHOLD, MAX_CONSECUTIVE_LOOPS
)


class LoopDetector:
    """Detects and prevents various types of loops in AI behavior."""
    
    def __init__(self, action_names):
        self.action_names = action_names
        
        # Frame-based loop detection
        self._frame_history = []
        self._frame_history_size = FRAME_HISTORY_SIZE
        self._loop_detection_window = LOOP_DETECTION_WINDOW
        
        # Action-based loop detection
        self._recent_actions = []
        self._action_history_size = ACTION_HISTORY_SIZE
        self._action_loop_threshold = ACTION_LOOP_THRESHOLD
        
        # Loop tracking and penalties
        self._consecutive_loops = 0
        self._max_consecutive_loops = MAX_CONSECUTIVE_LOOPS
        self._loop_penalty_escalation = 0.0
        self._last_loop_detection = 0
        
        # Movement tracking
        self._steps_without_movement = 0
    
    def detect_and_penalize_loops(self, obs, frame_counter):
        """
        Main loop detection method that combines all loop detection types.
        
        Args:
            obs: Current observation frame
            frame_counter: Current frame number
            
        Returns:
            float: Penalty value (negative) if loops detected, 0.0 otherwise
        """
        reward = 0.0
        
        # Create frame hash for visual loop detection
        loop_sample = obs[::4, ::4]  # Downsample for efficiency
        current_frame_hash = hash(loop_sample.tobytes())
        
        # Add current frame to history
        self._frame_history.append(current_frame_hash)
        if len(self._frame_history) > self._frame_history_size:
            self._frame_history.pop(0)
        
        # Detect different types of loops
        visual_loop_penalty = self._detect_visual_loops(frame_counter)
        movement_loop_penalty = self._detect_movement_loops(obs, frame_counter)
        
        reward += visual_loop_penalty
        reward += movement_loop_penalty
        
        # Update loop statistics and escalation
        if visual_loop_penalty < 0 or movement_loop_penalty < 0:
            self._consecutive_loops += 1
            self._last_loop_detection = frame_counter
            
            # Escalating penalties for persistent loops
            if self._consecutive_loops > self._max_consecutive_loops:
                self._loop_penalty_escalation = min(self._loop_penalty_escalation + 0.1, 1.0)
                reward -= self._loop_penalty_escalation
                
                if self._consecutive_loops % 10 == 0:
                    print(f"[Loop] 🚨 Persistent looping detected! "
                          f"Consecutive loops: {self._consecutive_loops}, "
                          f"Escalation penalty: -{self._loop_penalty_escalation:.2f}")
        else:
            # Reset loop counters if no loops detected (with grace period)
            if frame_counter - self._last_loop_detection > 50:
                self._consecutive_loops = max(0, self._consecutive_loops - 1)
                self._loop_penalty_escalation = max(0.0, self._loop_penalty_escalation - 0.05)
        
        return reward
    
    def _detect_visual_loops(self, frame_counter):
        """
        Detect visual loops by looking for repeating frame patterns.
        
        Args:
            frame_counter: Current frame number
            
        Returns:
            float: Negative penalty if loops detected, 0.0 otherwise
        """
        reward = 0.0
        
        if len(self._frame_history) >= self._loop_detection_window:
            # Check for exact frame repeats in recent history
            recent_frames = self._frame_history[-self._loop_detection_window:]
            
            # Count occurrences of each frame hash
            frame_counts = {}
            for frame_hash in recent_frames:
                frame_counts[frame_hash] = frame_counts.get(frame_hash, 0) + 1
            
            # Check for repeated frames
            max_repeats = max(frame_counts.values())
            
            if max_repeats >= 3:  # Same frame appeared 3+ times
                reward -= 0.3
                if max_repeats >= 4:  # Very repetitive
                    reward -= 0.5
                    
                if frame_counter % 100 == 0:  # Occasional logging
                    print(f"[Loop] 🔄 Visual loop detected! Frame repeated {max_repeats} times (-{-reward:.1f})")
            
            # Check for alternating patterns (A-B-A-B)
            if len(recent_frames) >= 4:
                if (recent_frames[-4] == recent_frames[-2] and 
                    recent_frames[-3] == recent_frames[-1] and
                    recent_frames[-4] != recent_frames[-3]):
                    reward -= 0.2
                    if frame_counter % 150 == 0:
                        print(f"[Loop] ↔️ Alternating pattern detected! (-0.2)")
        
        return reward
    
    def _detect_movement_loops(self, obs, frame_counter):
        """
        Detect movement loops by tracking position changes.
        
        Args:
            obs: Current observation frame
            frame_counter: Current frame number
            
        Returns:
            float: Negative penalty if stuck, 0.0 otherwise
        """
        reward = 0.0
        
        # This method needs a previous observation to compare
        # The actual movement comparison should be done in the calling code
        # This is a placeholder for movement-specific loop detection logic
        
        return reward
    
    def track_action(self, action, frame_counter):
        """
        Track the action taken for action loop detection.
        
        Args:
            action: Action taken
            frame_counter: Current frame number
            
        Returns:
            float: Penalty for action loops, 0.0 otherwise
        """
        # Add action to history
        self._recent_actions.append(action)
        if len(self._recent_actions) > self._action_history_size:
            self._recent_actions.pop(0)
        
        # Check for action loops (same action repeated too many times)
        if len(self._recent_actions) >= self._action_loop_threshold:
            recent_actions = self._recent_actions[-self._action_loop_threshold:]
            
            # Check if all recent actions are the same
            if len(set(recent_actions)) == 1:  # All actions are identical
                repeated_action = recent_actions[0]
                action_name = (self.action_names[repeated_action] 
                             if repeated_action < len(self.action_names) 
                             else "Unknown")
                
                if frame_counter % 100 == 0:  # Occasional logging
                    print(f"[Loop] 🔁 Action loop detected! "
                          f"Repeating '{action_name}' {self._action_loop_threshold}+ times")
                
                return -0.2  # Penalty for action loops
        
        return 0.0
    
    def update_movement_tracking(self, movement_magnitude):
        """
        Update movement tracking for stuck detection.
        
        Args:
            movement_magnitude: Amount of movement detected
        """
        if movement_magnitude < 0.5:
            self._steps_without_movement += 1
        else:
            self._steps_without_movement = 0
    
    def get_movement_penalty(self, frame_counter):
        """
        Get penalty for being stuck without movement.
        
        Args:
            frame_counter: Current frame number
            
        Returns:
            float: Movement penalty (negative value)
        """
        reward = 0.0
        
        # Escalating penalties for being stuck
        if self._steps_without_movement > 100:
            reward -= 0.1
        if self._steps_without_movement > 200:
            reward -= 0.2
        if self._steps_without_movement > 400:
            reward -= 0.5  # Heavy penalty for being very stuck
            
            if self._steps_without_movement % 200 == 0:
                print(f"[Loop] 🛑 Movement loop detected! "
                      f"Stuck for {self._steps_without_movement} frames (-{-reward:.1f})")
        
        return reward
    
    def reset(self):
        """Reset all loop detection state."""
        self._frame_history.clear()
        self._recent_actions.clear()
        self._consecutive_loops = 0
        self._loop_penalty_escalation = 0.0
        self._last_loop_detection = 0
        self._steps_without_movement = 0
        print("[Loop] 🔄 Loop detection system reset")
    
    def get_stats(self):
        """Get current loop detection statistics."""
        return {
            'consecutive_loops': self._consecutive_loops,
            'penalty_escalation': self._loop_penalty_escalation,
            'steps_without_movement': self._steps_without_movement,
            'frame_history_size': len(self._frame_history),
            'action_history_size': len(self._recent_actions)
        } 