"""
Device detection and configuration utilities for Pokemon Shiny Bot
Handles GPU/CPU detection and optimization settings
"""

from config import (
    GPU_BATCH_SIZE, GPU_N_STEPS, MPS_BATCH_SIZE, MPS_N_STEPS, 
    CPU_BATCH_SIZE, CPU_N_STEPS
)

try:
    import torch
except ModuleNotFoundError:
    torch = None


def detect_device():
    """
    Detect and configure the best available device (GPU/CPU).
    
    Returns:
        str: Device identifier ('cuda', 'mps', or 'cpu')
    """
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


def get_optimal_training_params(device):
    """
    Get optimal training parameters based on device capabilities.
    
    Args:
        device: Device identifier ('cuda', 'mps', or 'cpu')
        
    Returns:
        dict: Training parameters (batch_size, n_steps)
    """
    if device == "cuda":
        # GPU can handle larger batches
        batch_size = GPU_BATCH_SIZE
        n_steps = GPU_N_STEPS
        print(f"[GPU] 🚀 Optimized settings: batch_size={batch_size}, n_steps={n_steps}")
    elif device == "mps":
        # Apple Silicon optimization
        batch_size = MPS_BATCH_SIZE
        n_steps = MPS_N_STEPS
        print(f"[MPS] 🍎 Optimized settings: batch_size={batch_size}, n_steps={n_steps}")
    else:
        # CPU settings (smaller batches)
        batch_size = CPU_BATCH_SIZE
        n_steps = CPU_N_STEPS
        print(f"[CPU] 💻 CPU settings: batch_size={batch_size}, n_steps={n_steps}")
    
    return {
        'batch_size': batch_size,
        'n_steps': n_steps
    }


def is_torch_available():
    """Check if PyTorch is available."""
    return torch is not None


def get_device_info(device):
    """
    Get detailed information about the selected device.
    
    Args:
        device: Device identifier
        
    Returns:
        dict: Device information
    """
    info = {'device': device, 'available': False}
    
    if torch is None:
        info['name'] = 'PyTorch not available'
        return info
    
    info['available'] = True
    
    if device == "cuda" and torch.cuda.is_available():
        info['name'] = torch.cuda.get_device_name(0)
        info['memory_gb'] = torch.cuda.get_device_properties(0).total_memory / 1024**3
        info['compute_capability'] = torch.cuda.get_device_capability(0)
    elif device == "mps" and torch.backends.mps.is_available():
        info['name'] = 'Apple Silicon GPU (MPS)'
        info['memory_gb'] = 'Shared with system RAM'
    else:
        info['name'] = 'CPU'
        info['memory_gb'] = 'System RAM'
    
    return info 