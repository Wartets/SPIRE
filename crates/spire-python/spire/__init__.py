"""
SPIRE - Structured Particle Interaction & Reaction Engine
==========================================================

High-performance Python bindings for the SPIRE QFT / HEP physics kernel,
compiled from Rust via PyO3 with zero-copy NumPy interoperability.

Quick Start
-----------
>>> import spire
>>> sm = spire.Model.standard_model()
>>> rxn = spire.Reaction.create(sm, ["e-", "e+"], ["mu-", "mu+"], cms_energy=91.2)
>>> print(rxn.is_valid)
True

Event Generation (NumPy)
-------------------------
>>> gen = spire.EventGenerator(cms_energy=91.2, final_masses=[0.1057, 0.1057], seed=42)
>>> momenta, weights = gen.generate(100_000)
>>> print(momenta.shape)   # (100000, 2, 4)
"""

from __future__ import annotations

# Import all public symbols from the compiled native extension.
from spire._native import (
    AmplitudeResult,
    CrossSectionResult,
    DecayChannel,
    DecayTable,
    DiagramSet,
    EventGenerator,
    Field,
    HadronicCrossSectionResult,
    Model,
    Particle,
    Reaction,
    calculate_threshold,
    is_kinematically_allowed,
)

__all__ = [
    "AmplitudeResult",
    "CrossSectionResult",
    "DecayChannel",
    "DecayTable",
    "DiagramSet",
    "EventGenerator",
    "Field",
    "HadronicCrossSectionResult",
    "Model",
    "Particle",
    "Reaction",
    "calculate_threshold",
    "is_kinematically_allowed",
]

__version__ = "0.1.0"
