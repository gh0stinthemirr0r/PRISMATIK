"""
Core chart type definitions.
"""

from __future__ import annotations
from dataclasses import dataclass, field
from datetime import datetime
from decimal import Decimal
from typing import Optional
import attr


@attr.s(auto_attribs=True, frozen=False)
class CandlestickPoint:
    """
    A single candlestick/bar chart data point.
    
    Attributes:
        timestamp: The time of the candle (ISO format string or datetime)
        open: Opening price
        high: Highest price during period
        low: Lowest price during period  
        close: Closing price
        volume: Trading volume for the period
        otc_volume: Optional OTX/OTC exchange specific volume metric
        wicks_visible: Whether to show upper/lower shadow lines
        split_points: Optional list of (price, level) tuples for horizontal splits
    """
    timestamp: datetime | str
    open: Decimal | float
    high: Decimal | float
    low: Decimal | float
    close: Decimal | float
    volume: Decimal | int = 0
    otc_volume: Optional[Decimal] = None
    wicks_visible: bool = True
    split_points: list[tuple[float, str]] | None = field(default=None)

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON serialization."""
        return {
            "timestamp": self.timestamp.isoformat() if isinstance(self.timestamp, datetime) else self.timestamp,
            "open": float(self.open),
            "high": float(self.high),
            "low": float(self.low),
            "close": float(self.close),
            "volume": int(self.volume) if Decimal(str(self.volume)) != int(self.volume) else int(self.volume),
            "otc_volume": float(self.otc_volume) if self.otc_volume is not None else None,
            "wicks_visible": self.wicks_visible,
            "split_points": [{"price": p, "level": l} for p, l in (self.split_points or [])],
        }


@attr.s(auto_attribs=True)
class SplitLine:
    """
    A horizontal split line on the chart.
    
    Attributes:
        price: The price level
        label: Display text for the line (optional)
        style: Line style - 'solid', 'dashed', or 'dotted'
        color: Hex color string
        visible: Whether this line should be rendered
    """
    price: float
    label: Optional[str] = None
    style: str = "solid"
    color: str = "#ffffff"
    visible: bool = True