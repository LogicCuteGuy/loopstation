//! Unit tests for loopstation core components
//! 
//! This module contains comprehensive tests for:
//! - Track audio buffer management and state transitions
//! - EffectChain processing and parameter control  
//! - MemorySystem project save/load functionality

#[cfg(test)]
mod simple_tests;

#[cfg(test)]
mod communication_tests;

#[cfg(test)]
mod integration_simple;

#[cfg(test)]
mod simple_communication_tests;

#[cfg(test)]
mod modulation_tests;

#[cfg(test)]
mod performance_tests;