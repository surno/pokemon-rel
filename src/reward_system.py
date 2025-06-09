"""
Reward system for Pokemon Shiny Bot
Calculates rewards for various gameplay behaviors to encourage good AI play
"""

import numpy as np
from config import MAX_STEPS_STILL


class PokemonRewardSystem:
    """Calculates rewards for Pokemon-specific gameplay behaviors."""
    
    def __init__(self):
        # State tracking for reward calculations
        self._last_obs = None
        self._visited_areas = set()
        self._steps_without_movement = 0
        self._max_steps_still = MAX_STEPS_STILL
        
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
        
        # Screen state analysis
        self._menu_state = False
        self._battle_menu_state = False
        self._overworld_state = True
        self._menu_frames_count = 0
    
    def compute_reward(self, obs):
        """
        Main reward computation method.
        
        Pokemon-specific reward system that encourages proper gameplay behaviors.
        
        Reward Structure:
        🎮 Basic Gameplay (Reliable Detection):
        - 🚶 Movement/exploration: +0.1 to +0.2
        - 🗺️  New area discovery: +0.5
        - ⏱️  Time penalty for standing still: -0.05 to -0.1
        
        🔬 Advanced Detection (Conservative):
        - 🎣 Potential Pokemon encounters: +1.0 (when screen changes dramatically)
        - ⚔️  Potential battle events: +0.3 (when in battle-like state)
        
        Args:
            obs: Current observation frame
            
        Returns:
            float: Total reward value
        """
        reward = 0.0
        
        if self._last_obs is not None:
            # Focus on reliable, simple detection
            reward += self._detect_menu_states_and_escape(obs)       # Menu handling
            reward += self._detect_movement_and_exploration(obs)     # Movement rewards
            reward += self._detect_screen_transitions(obs)           # Major screen changes
            reward += self._detect_potential_battles(obs)            # Conservative battle detection
            reward += self._apply_idle_penalty(obs)                  # Standing still penalty
        
        # Store current observation for next comparison
        self._last_obs = obs.copy()
        
        return reward
    
    def _detect_menu_states_and_escape(self, obs):
        """
        📋 Detect when the bot is stuck in menus and encourage escaping back to overworld.
        
        Pokemon games have many menus that can trap the bot:
        - Start menu (Pokédex, Pokemon, Bag, etc.)
        - Pokemon status screens
        - Item menus
        - Settings menus
        - Battle menus
        
        Returns:
            float: Reward/penalty for menu interactions
        """
        reward = 0.0
        
        # Detect if we're in a menu state
        current_menu_state = self._analyze_menu_state(obs)
        
        # Menu transition rewards/penalties
        if current_menu_state and not self._menu_state:
            # Just entered a menu - small penalty to discourage menu camping
            reward -= 0.1
            self._menu_frames_count = 0
            print(f"[Menu] 📋 Entered menu state (-0.1)")
            
        elif not current_menu_state and self._menu_state:
            # Successfully escaped a menu - REWARD!
            reward += 0.5
            print(f"[Menu] ✅ Escaped menu! Back to overworld (+0.5)")
            
        elif current_menu_state:
            # Still in menu - escalating penalties
            self._menu_frames_count = getattr(self, '_menu_frames_count', 0) + 1
            
            if self._menu_frames_count > 50:
                reward -= 0.2
            if self._menu_frames_count > 150:
                reward -= 0.4
            if self._menu_frames_count > 300:
                reward -= 0.6
                
            # Periodic reminders to escape
            if self._menu_frames_count % 100 == 0:
                print(f"[Menu] 🚨 Stuck in menu for {self._menu_frames_count} frames! "
                      f"Learn to press B to escape (-{-reward:.1f})")
        
        # Update menu state
        self._menu_state = current_menu_state
        
        return reward
    
    def _analyze_menu_state(self, obs):
        """
        Analyze the screen to determine if we're in a menu state.
        
        Menu characteristics in Pokemon games:
        - Often have solid color backgrounds (blue, white, etc.)
        - Text-heavy interfaces
        - Less color variance than overworld
        - Different UI layouts
        
        Args:
            obs: Current observation frame
            
        Returns:
            bool: True if in menu state, False otherwise
        """
        # Sample different areas of the screen
        top_area = obs[0:50, :]      # Top menu bar area
        center_area = obs[60:130, :] # Main content area  
        bottom_area = obs[140:, :]   # Bottom area
        
        # Calculate color characteristics
        top_variance = np.var(top_area)
        center_variance = np.var(center_area)
        bottom_variance = np.var(bottom_area)
        
        # Calculate average brightness
        avg_brightness = np.mean(obs)
        
        # Menu detection heuristics
        menu_indicators = 0
        
        # 1. Low variance in large areas (solid backgrounds)
        if center_variance < 200:
            menu_indicators += 1
            
        # 2. High contrast areas (white text on colored backgrounds)
        if top_variance > 800 or bottom_variance > 800:
            menu_indicators += 1
            
        # 3. Overall brightness patterns (menus often brighter)
        if avg_brightness > 120:
            menu_indicators += 1
            
        # 4. Check for menu-like color patterns
        # Look for dominant single colors (common in menu backgrounds)
        unique_colors = len(np.unique(obs.reshape(-1, obs.shape[-1]), axis=0))
        if unique_colors < 50:  # Very few unique colors = likely menu
            menu_indicators += 1
        
        # 5. Text detection (menus have lots of text)
        # Look for horizontal line patterns that suggest text
        horizontal_edges = np.sum(np.abs(np.diff(obs, axis=0)))
        if horizontal_edges > 200000:  # High horizontal edge activity
            menu_indicators += 1
            
        # Consider it a menu if 3+ indicators are present
        is_menu = menu_indicators >= 3
        
        return is_menu
    
    def _detect_movement_and_exploration(self, obs):
        """
        🚶 Simple, reliable movement and exploration detection.
        This is the most reliable way to encourage good gameplay.
        
        Args:
            obs: Current observation frame
            
        Returns:
            float: Movement and exploration rewards
        """
        reward = 0.0
        
        # Calculate movement by comparing with last frame
        movement_diff = np.abs(obs.astype(np.float32) - self._last_obs.astype(np.float32))
        movement_amount = np.mean(movement_diff)
        
        # Only reward movement when NOT in a menu (overworld movement)
        if movement_amount > 2.0 and not getattr(self, '_menu_state', False):
            reward += 0.2  # Reward for overworld movement
            self._steps_without_movement = 0
            
            print(f"[Pokemon] 🚶 Character moving in overworld! (+0.2)")
            
            # Check for new area discovery (more conservative)
            area_sample = obs[70:150, 70:186]  # Focused area sample
            area_hash = hash(area_sample[::6, ::6].tobytes())  # Downsampled for stability
            
            if area_hash not in self._visited_areas:
                reward += 0.8  # Higher bonus for overworld exploration
                self._visited_areas.add(area_hash)
                print(f"[Pokemon] 🗺️ New overworld area discovered! (+0.8)")
                
                if len(self._visited_areas) % 15 == 0:
                    print(f"[Pokemon] 🌍 Explored {len(self._visited_areas)} unique overworld areas!")
                    
        elif movement_amount > 2.0 and getattr(self, '_menu_state', False):
            # Movement in menus gets much smaller reward (menu navigation)
            reward += 0.02  # Very small reward for menu navigation
        else:
            # No movement detected
            if not getattr(self, '_menu_state', False):
                # Standing still in overworld is bad
                self._steps_without_movement += 1
        
        return reward
    
    def _detect_screen_transitions(self, obs):
        """
        🔄 Detect major screen transitions that might indicate important events.
        Much more conservative than before.
        
        Args:
            obs: Current observation frame
            
        Returns:
            float: Screen transition rewards
        """
        reward = 0.0
        
        # Look for MAJOR screen changes (like entering battles, menus, etc.)
        if self._last_obs is not None:
            # Calculate the overall difference between frames
            total_diff = np.mean(np.abs(obs.astype(np.float32) - self._last_obs.astype(np.float32)))
            
            # Only trigger on VERY large changes (major screen transitions)
            if total_diff > 25.0:  # Much higher threshold
                reward += 1.0
                print(f"[Pokemon] 🔄 Major screen transition detected (+1.0)")
        
        return reward
    
    def _detect_potential_battles(self, obs):
        """
        ⚔️ Very conservative battle detection.
        Only triggers on obvious battle-like screen patterns.
        
        Args:
            obs: Current observation frame
            
        Returns:
            float: Battle detection rewards
        """
        reward = 0.0
        
        # Look for battle-like screen characteristics
        battle_area = obs[60:130, 40:216]
        
        # Check if the screen has battle-like characteristics
        avg_brightness = np.mean(battle_area)
        color_variance = np.var(battle_area)
        
        # Much more conservative detection
        is_battle_like = (
            avg_brightness < 60 and color_variance > 800  # Very specific conditions
        )
        
        current_battle_state = is_battle_like
        
        # Only reward when we're clearly in a battle state
        if current_battle_state and current_battle_state != self._last_battle_state:
            reward += 0.3  # Small reward for potential battle
            print(f"[Pokemon] ⚔️ Potential battle detected (+0.3)")
        
        self._last_battle_state = current_battle_state
        return reward
    
    def _apply_idle_penalty(self, obs):
        """
        🚫 Simple penalty for standing still too long.
        Much more conservative than before.
        
        Args:
            obs: Current observation frame
            
        Returns:
            float: Idle penalties
        """
        reward = 0.0
        
        # Calculate if we're standing still
        if self._last_obs is not None:
            movement_diff = np.mean(np.abs(obs.astype(np.float32) - self._last_obs.astype(np.float32)))
            
            if movement_diff < 1.5:  # Very little change
                self._steps_without_movement += 1
                
                # Very mild penalties
                if self._steps_without_movement > self._max_steps_still:
                    if self._steps_without_movement < self._max_steps_still + 100:
                        reward -= 0.05  # Very mild penalty
                    else:
                        reward -= 0.1   # Slightly stronger penalty
                        
                    # Less frequent logging
                    if self._steps_without_movement % 200 == 0:
                        print(f"[Pokemon] ⏱️ Idle penalty: {reward:.2f}")
        
        return reward
    
    # Disabled detection methods (were giving false positives)
    def _detect_pokemon_encounters(self, obs):
        """DISABLED - Was giving false positives"""
        return 0.0

    def _detect_shiny_pokemon(self, obs):
        """DISABLED - Was giving false positives"""
        return 0.0

    def _detect_battle_events(self, obs):
        """DISABLED - Was giving false positives"""
        return 0.0

    def _detect_level_ups(self, obs):
        """DISABLED - Was giving false positives"""
        return 0.0

    def _detect_items_found(self, obs):
        """DISABLED - Was giving false positives"""
        return 0.0
    
    def reset(self):
        """Reset reward system state."""
        self._last_obs = None
        self._visited_areas = set()
        self._steps_without_movement = 0
        
        # Reset Pokemon encounter and battle tracking
        self._last_battle_state = False
        self._battle_start_frame = 0
        self._wild_encounter_detected = False
        # Note: Keep cumulative counts across resets for long sessions
        
        # Reset game state analysis
        self._menu_state = False
        self._battle_menu_state = False
        self._overworld_state = True
        self._menu_frames_count = 0
        
        print("[Reward] 🔄 Reward system reset")
        print(f"[Reward] 📊 Session stats - Shinies: {self._shiny_encounters}, "
              f"Level ups: {self._level_ups_detected}, Items: {self._item_found_count}")
    
    def get_stats(self):
        """Get reward system statistics."""
        return {
            'visited_areas': len(self._visited_areas),
            'steps_without_movement': self._steps_without_movement,
            'shiny_encounters': self._shiny_encounters,
            'level_ups': self._level_ups_detected,
            'items_found': self._item_found_count,
            'menu_state': self._menu_state,
            'battle_state': self._last_battle_state
        } 