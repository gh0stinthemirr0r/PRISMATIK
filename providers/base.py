"""
Base Provider Abstraction Layer
Implements OpenBB-style interface for multi-source data integration.

Key Concepts:
- BaseProvider: Abstract base class with capability declaration support
- Capabilities: Metadata schema defining what data a provider offers
- CapabilityMatrix: Visualization contract showing provider coverage
- HealthMetrics: Uptime/latency tracking hooks
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Optional, Any, Protocol, runtime_checkable
import json
from datetime import datetime
from dataclasses import dataclass, field


@dataclass
class Capability:
    """Defines a capability (data type) exposed by a provider."""
    name: str
    endpoint: str  # API path or method
    description: str
    required_params: Optional[Dict[str, Any]] = None
    supported_formats: List[str] = field(default_factory=list)
    latency_target_ms: int = 1000  # SLA target
    
    def to_dict(self) -> Dict:
        return {
            'name': self.name,
            'endpoint': self.endpoint,
            'description': self.description,
            'required_params': self.required_params or {},
            'supported_formats': self.supported_formats,
            'latency_target_ms': self.latency_target_ms
        }


@dataclass
class HealthMetrics:
    """Tracks health metrics for a provider."""
    last_heartbeat: datetime = field(default_factory=datetime.now)
    uptime_seconds: float = 0.0
    latency_p99_ms: Optional[float] = None
    error_rate_pct: Optional[float] = None
    data_freshness_minutes: Optional[int] = None
    metadata_updates: int = 0


class Capabilities(Protocol):
    """Protocol defining capability declaration interface."""
    @property
    def name(self) -> str:
        ...
    
    @property
    def capabilities(self) -> List[Capability]:
        ...
    
    @property
    def health(self) -> HealthMetrics:
        ...


@runtime_checkable
class BaseProvider(Protocol):
    """
    Abstract provider interface.
    All providers must implement this to be discovered and composed.
    """
    name: str
    version: str
    description: str
    base_url: Optional[str] = None
    
    # Capabilities (data endpoints)
    @property
    def capabilities(self) -> List[Capability]:
        """Return list of capability descriptors."""
        ...
    
    # Health hooks
    def get_health(self) -> Dict[str, Any]:
        """Return current health status for monitoring."""
        ...
    
    def probe(self) -> bool:
        """Lightweight liveness check (no data fetch)."""
        ...
    
    # Data access
    async def query(self, capability: Capability, params: Optional[Dict] = None) -> Any:
        """Execute a data query against an endpoint."""
        ...


class ProviderRegistry:
    """Registry for discovering and managing providers."""
    
    def __init__(self):
        self._providers: Dict[str, BaseProvider] = {}
        self._discovery_endpoints: List[str] = []  # For automatic discovery
        
    def register(self, provider: BaseProvider) -> None:
        """Register a provider instance."""
        key = f"{provider.name}:{provider.version}"
        self._providers[key] = provider
    
    def unregister(self, name_version: str) -> bool:
        """Remove provider from registry."""
        if name_version in self._providers:
            del self._providers[name_version]
            return True
        return False
    
    def get(self, key: str) -> Optional[BaseProvider]:
        """Retrieve provider by name:version."""
        return self._providers.get(key)
    
    def list_all(self) -> List[Dict[str, Any]]:
        """List all registered providers with metadata."""
        result = []
        for key, provider in sorted(self._providers.items()):
            cap_list = [cap.to_dict() for cap in provider.capabilities]
            health = provider.get_health()
            result.append({
                'key': key,
                'name': provider.name,
                'version': provider.version,
                'description': provider.description,
                'capabilities': len(cap_list),
                'base_url': provider.base_url,
                'health': health
            })
        return result
    
    def discover(self, endpoint: str, method: str = 'GET') -> List[str]:
        """
        Discover available providers from a remote registry.
        Returns list of discovered provider keys (name:version).
        """
        # In production, this would actually fetch from the endpoint
        return self._discovery_endpoints + [endpoint]
    
    def health_check_all(self) -> Dict[str, Any]:
        """Aggregate health status across all providers."""
        total = len(self._providers)
        healthy = 0
        unhealthy = 0
        avg_latency_ms = 0.0
        
        for provider in self._providers.values():
            try:
                if provider.probe():
                    health = provider.get_health()
                    healthy += 1
                    # Aggregate latency metrics
                    lat = getattr(health, 'latency_p99_ms', None)
                    if lat is not None:
                        avg_latency_ms += lat
            except Exception as e:
                unhealthy += 1
        
        return {
            'total': total,
            'healthy': healthy,
            'unhealthy': unhealthy,
            'availability_pct': round((healthy / max(total, 1)) * 100, 2),
            'avg_latency_ms': round(avg_latency_ms / max(healthy, 1), 2)
        }


def create_capability_matrix(registry: Optional[ProviderRegistry] = None) -> Dict[str, Any]:
    """
    Generate capability matrix visualization contract.
    Shows which capabilities are supported across providers.
    
    Returns:
        Dictionary with provider coverage matrix suitable for rendering
        as markdown tables or JSON visualization specs.
    """
    if registry is None:
        registry = ProviderRegistry()
    
    # Get all capability names from all providers
    all_caps: Dict[str, List[Capability]] = {}
    
    for provider in registry.list_all():
        caps = provider.get('capabilities', [])
        cap_names = [cap['name'] for cap in caps]
        if not cap_names:
            continue
        
        # Aggregate by capability name (union of providers supporting it)
        for cap_name in set(cap_names):
            if cap_name not in all_caps:
                all_caps[cap_name] = {'providers': [], 'formats': set(), 'sources': []}
            provider_info = {
                'name': provider['name'],
                'version': provider['version'],
                'endpoint': provider.get('base_url', ''),
                'latency_target_ms': provider.get('capabilities', [{}])[0].get('latency_target_ms', 1000)
            }
            all_caps[cap_name]['providers'].append(provider_info)
    
    # Build matrix data
    capability_names = sorted(all_caps.keys())
    provider_names = list(set(p['name'] for p in [v for v in all_caps.values()]))
    
    matrix_data = {
        'capabilities': capability_names,
        'providers': provider_names,
        'coverage': {}
    }
    
    # Fill coverage
    for cap_name in capability_names:
        entry = all_caps[cap_name]
        providers = entry['providers']
        formats = set()
        sources = []
        
        for prov in providers:
            cap_key = f"{prov['name']}:{prov['version']}"
            registry_providers = [p for k, p in registry._providers.items() if k == cap_key]
            if registry_providers:
                provider_obj = registry_providers[0]
                for capability in provider_obj.capabilities:
                    if capability.name == cap_name:
                        formats.update(capability.supported_formats)
                        sources.append(f"{prov['name']}@{prov.get('base_url', '')}")
        
        matrix_data['coverage'][cap_name] = {
            'sources': providers,
            'formats': list(formats),
            'source_urls': sources
        }
    
    return matrix_data