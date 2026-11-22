"""apifuzz — анализ результатов фuzz-тестов API."""

from .report import VERSION, analyze, load_results

__all__ = ["VERSION", "analyze", "load_results"]
