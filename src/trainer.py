"""
AI Training module for Pokemon Shiny Bot
Handles PPO training setup and execution
"""

import time
import numpy as np
from config import DEFAULT_TOTAL_TIMESTEPS
from device_utils import detect_device, get_optimal_training_params, is_torch_available
from network import shutdown_requested, shutdown_start_time

try:
    from stable_baselines3 import PPO
except ModuleNotFoundError:
    PPO = None


class PokemonTrainer:
    """Handles training of the Pokemon shiny hunting AI."""
    
    def __init__(self, environment):
        self.env = environment
        self.model = None
        self.device = None
        self.training_params = None
        
        # Initialize if PPO is available
        if PPO is not None and is_torch_available():
            self._setup_training()
    
    def _setup_training(self):
        """Set up the training environment and model."""
        # Detect optimal device and parameters
        self.device = detect_device()
        self.training_params = get_optimal_training_params(self.device)
        
        # Create PPO model with device-specific optimization
        self.model = PPO(
            "CnnPolicy", 
            self.env, 
            device=self.device,
            verbose=1,
            learning_rate=3e-4,
            n_steps=self.training_params['n_steps'],
            batch_size=self.training_params['batch_size'],
            n_epochs=10,
            gamma=0.99,
            gae_lambda=0.95,
            clip_range=0.2,
            ent_coef=0.01,  # Encourage exploration
            vf_coef=0.5,
            max_grad_norm=0.5,
            seed=42
        )
        
        print("🤖 ROBUST TRAINING MODE - Overnight AI training...")
        print("🛡️  Features: Auto-reconnect, Error recovery, Graceful shutdown")
        print("Press Ctrl+C for graceful shutdown")
    
    def is_training_available(self):
        """Check if training is available (PPO and PyTorch installed)."""
        return PPO is not None and is_torch_available() and self.model is not None
    
    def train(self, total_timesteps=None):
        """
        Train the AI model.
        
        Args:
            total_timesteps: Total training steps (uses default if None)
            
        Returns:
            bool: True if training completed successfully, False if interrupted
        """
        if not self.is_training_available():
            print("⚠️  Training not available - PPO or PyTorch not installed")
            return False
        
        if total_timesteps is None:
            total_timesteps = DEFAULT_TOTAL_TIMESTEPS
        
        print("🚀 Starting interruptible training...")
        remaining_timesteps = total_timesteps
        
        try:
            while remaining_timesteps > 0 and not shutdown_requested:
                # Train in smaller chunks to allow interruption
                chunk_size = min(2048, remaining_timesteps)
                
                # Check for shutdown timeout (force quit after 10 seconds)
                if shutdown_requested and shutdown_start_time:
                    elapsed = time.time() - shutdown_start_time
                    if elapsed > 10.0:
                        print("[Trainer] ⏰ Shutdown timeout - forcing exit...")
                        break
                
                self.model.learn(total_timesteps=chunk_size)
                remaining_timesteps -= chunk_size
                
                progress = total_timesteps - remaining_timesteps
                print(f"[Training] 📊 Progress: {progress}/{total_timesteps} timesteps")
            
            if shutdown_requested:
                print("🛑 Training interrupted by shutdown request")
                return False
            else:
                print("🎉 Training completed successfully!")
                return True
                
        except KeyboardInterrupt:
            print("\n🛑 Training interrupted by user")
            return False
        except Exception as e:
            print(f"❌ Training error: {e}")
            print("🔄 Training can be resumed from checkpoint")
            return False
    
    def save_model(self, path):
        """
        Save the trained model.
        
        Args:
            path: Path to save the model
        """
        if self.model is not None:
            self.model.save(path)
            print(f"[Trainer] 💾 Model saved to {path}")
        else:
            print("[Trainer] ⚠️  No model to save")
    
    def load_model(self, path):
        """
        Load a trained model.
        
        Args:
            path: Path to load the model from
        """
        if PPO is not None:
            try:
                self.model = PPO.load(path, env=self.env, device=self.device)
                print(f"[Trainer] 📂 Model loaded from {path}")
                return True
            except Exception as e:
                print(f"[Trainer] ❌ Failed to load model: {e}")
                return False
        else:
            print("[Trainer] ⚠️  PPO not available - cannot load model")
            return False
    
    def test_model(self, num_steps=1000):
        """
        Test the trained model.
        
        Args:
            num_steps: Number of steps to test
        """
        if self.model is None:
            print("[Trainer] ⚠️  No model available for testing")
            return
        
        print(f"[Trainer] 🧪 Testing model for {num_steps} steps...")
        
        obs = self.env.reset()
        total_reward = 0
        
        for step in range(num_steps):
            if shutdown_requested:
                break
                
            action, _states = self.model.predict(obs, deterministic=True)
            obs, reward, done, info = self.env.step(action)
            total_reward += reward
            
            if done:
                obs = self.env.reset()
                
            if step % 100 == 0:
                print(f"[Test] Step {step}, Total reward: {total_reward:.2f}")
        
        avg_reward = total_reward / num_steps
        print(f"[Trainer] 📊 Test completed. Average reward: {avg_reward:.3f}")
    
    def get_model_info(self):
        """Get information about the current model."""
        if self.model is None:
            return {"status": "No model loaded"}
        
        return {
            "status": "Model ready",
            "device": self.device,
            "policy": "CnnPolicy",
            "algorithm": "PPO",
            "training_params": self.training_params
        }


class TestRunner:
    """Handles test mode execution when training is not available."""
    
    def __init__(self, environment):
        self.env = environment
    
    def run_test_mode(self):
        """
        Run in test mode with random actions.
        Useful for overnight monitoring or when PPO is not available.
        """
        print("🎮 TESTING MODE - Overnight frame monitoring...")
        print("Press Ctrl+C to stop gracefully")
        
        frame_count = 0
        start_time = time.time()
        
        try:
            while not shutdown_requested:
                # Send mostly nothing, occasionally move
                action = (0 if frame_count % 15 != 0 
                         else np.random.randint(0, len(self.env.action_names)))
                
                obs, reward, done, info = self.env.step(action)
                frame_count += 1
                
                # Log progress less frequently for overnight operation
                if frame_count % 1000 == 0:
                    uptime = time.time() - start_time
                    fps = frame_count / uptime if uptime > 0 else 0
                    print(f"[Test Mode] 📊 {frame_count} frames, {uptime:.1f}s uptime, {fps:.1f} FPS")
                
                # Check for shutdown timeout
                if shutdown_requested and shutdown_start_time:
                    elapsed = time.time() - shutdown_start_time
                    if elapsed > 5.0:  # Shorter timeout for test mode
                        print("[Test Mode] ⏰ Shutdown timeout - exiting...")
                        break
                
                # Handle done conditions
                if done:
                    if info.get("fatal_error"):
                        print("[Test Mode] 💥 Fatal error detected")
                        break
                    elif info.get("shutdown"):
                        print("[Test Mode] 🛑 Shutdown requested")
                        break
                        
        except KeyboardInterrupt:
            print("\n[Test Mode] 🛑 Stopped by user (Ctrl+C)")
        except Exception as e:
            print(f"[Test Mode] ❌ Error: {e}")
            print("[Test Mode] 🔄 Attempting to continue...")
            time.sleep(1)
        
        # Calculate final stats
        total_time = time.time() - start_time
        avg_fps = frame_count / total_time if total_time > 0 else 0
        
        print(f"[Test Mode] 📊 Final stats: {frame_count} frames in {total_time:.1f}s (avg {avg_fps:.1f} FPS)")


def create_trainer(environment, training_mode=True):
    """
    Factory function to create appropriate trainer based on availability.
    
    Args:
        environment: The Pokemon environment
        training_mode: Whether to prefer training mode over test mode
        
    Returns:
        Either PokemonTrainer or TestRunner
    """
    if training_mode and PPO is not None and is_torch_available():
        return PokemonTrainer(environment)
    else:
        return TestRunner(environment) 