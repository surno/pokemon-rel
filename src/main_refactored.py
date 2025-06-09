#!/usr/bin/env python3
"""
Pokemon Shiny Bot - Main Entry Point
A reinforcement learning bot for Pokemon shiny hunting

This is the refactored, modular version that's much easier to read and maintain.
"""

import sys
from pokemon_environment import PokemonEnvironment
from trainer import create_trainer
from network import setup_shutdown_handler, shutdown_requested


def main():
    """Main entry point for the Pokemon Shiny Bot."""
    print("🎮 Pokemon Shiny Bot - Refactored Version")
    print("=" * 50)
    
    # Set up graceful shutdown handling
    setup_shutdown_handler()
    
    # Configuration options
    SKIP_TRAINING = False  # Set to True to run in test mode only
    
    try:
        # Create the Pokemon environment
        print("🔧 Initializing Pokemon environment...")
        env = PokemonEnvironment()
        
        # Create appropriate trainer (AI training or test mode)
        trainer = create_trainer(env, training_mode=not SKIP_TRAINING)
        
        # Run the appropriate mode
        if hasattr(trainer, 'is_training_available') and trainer.is_training_available():
            # AI Training Mode
            print("🤖 Starting AI training mode...")
            success = trainer.train()
            
            if success:
                # Optionally save the model
                model_path = "pokemon_shiny_model"
                trainer.save_model(model_path)
                
                # Test the trained model
                print("🧪 Testing trained model...")
                trainer.test_model(num_steps=500)
        else:
            # Test Mode (no AI training)
            print("🎮 Starting test mode...")
            trainer.run_test_mode()
    
    except KeyboardInterrupt:
        print("\n🛑 Interrupted by user")
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
        print("💡 Check your setup and try again")
    
    finally:
        # Cleanup
        print("\n🧹 Cleaning up...")
        try:
            if 'env' in locals():
                env.close()
        except:
            pass
        
        print("✅ Cleanup complete - Good night! 🌙")


def run_specific_mode():
    """
    Alternative entry point for running specific modes.
    Useful for development and testing.
    """
    import argparse
    
    parser = argparse.ArgumentParser(description='Pokemon Shiny Bot')
    parser.add_argument('--mode', choices=['train', 'test', 'load'], 
                       default='train', help='Operation mode')
    parser.add_argument('--timesteps', type=int, default=100000,
                       help='Training timesteps')
    parser.add_argument('--model-path', type=str, default='pokemon_model',
                       help='Model save/load path')
    
    args = parser.parse_args()
    
    # Set up graceful shutdown
    setup_shutdown_handler()
    
    try:
        env = PokemonEnvironment()
        
        if args.mode == 'train':
            trainer = create_trainer(env, training_mode=True)
            if hasattr(trainer, 'train'):
                trainer.train(total_timesteps=args.timesteps)
                trainer.save_model(args.model_path)
            else:
                print("Training not available, falling back to test mode")
                trainer.run_test_mode()
                
        elif args.mode == 'test':
            trainer = create_trainer(env, training_mode=False)
            trainer.run_test_mode()
            
        elif args.mode == 'load':
            trainer = create_trainer(env, training_mode=True)
            if hasattr(trainer, 'load_model'):
                if trainer.load_model(args.model_path):
                    trainer.test_model(num_steps=1000)
                else:
                    print("Failed to load model, falling back to test mode")
                    test_trainer = create_trainer(env, training_mode=False)
                    test_trainer.run_test_mode()
            else:
                print("Model loading not available")
    
    except Exception as e:
        print(f"Error: {e}")
    finally:
        if 'env' in locals():
            env.close()


if __name__ == "__main__":
    # Check if command line arguments are provided
    if len(sys.argv) > 1:
        run_specific_mode()
    else:
        main() 