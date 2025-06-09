"""
Pokemon Shiny Bot - Refactored Modular Version

This package contains a cleanly refactored version of the Pokemon shiny hunting bot.
The original 1,130-line main.py has been broken down into focused, maintainable modules.

Modules:
- config.py: All configuration settings and constants
- network.py: TCP communication with the Lua script
- image_processing.py: Image conversion and display utilities
- device_utils.py: GPU/CPU detection and optimization
- reward_system.py: Pokemon gameplay reward calculations
- loop_detection.py: Loop detection and prevention system
- pokemon_environment.py: Main OpenAI Gym environment
- trainer.py: AI training and test mode logic
- main_refactored.py: Clean main entry point

Usage:
    python main_refactored.py              # Run with default settings
    python main_refactored.py --mode test  # Run in test mode only
    python main_refactored.py --mode train --timesteps 50000  # Custom training
"""

__version__ = "2.0.0"
__author__ = "Pokemon Shiny Bot Team"

# Export main classes for easy importing
from .pokemon_environment import PokemonEnvironment
from .trainer import PokemonTrainer, TestRunner, create_trainer
from .reward_system import PokemonRewardSystem
from .loop_detection import LoopDetector
from .network import NetworkManager

__all__ = [
    'PokemonEnvironment',
    'PokemonTrainer', 
    'TestRunner',
    'create_trainer',
    'PokemonRewardSystem',
    'LoopDetector',
    'NetworkManager'
] 