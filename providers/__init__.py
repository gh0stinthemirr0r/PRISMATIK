# Provider Abstraction Layer for Multi-Source Data Integration
# OpenBB-style pattern implementation

from .base import (
    BaseProvider,
    ProviderRegistry,
    Capabilities, 
    HealthMetrics,
    CapabilityMatrix
)
from .registry import register_provider, discover_providers
from .health import HealthMonitor, HealthHook

__version__ = "2.0.0"
__all__ = [
    # Core abstractions
    'BaseProvider',
    'ProviderRegistry', 
    'Capabilities',
    'HealthMetrics',
    'CapabilityMatrix',
    # Utilities
    'register_provider',
    'discover_providers',
    'HealthMonitor',
    'HealthHook'
]
