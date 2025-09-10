# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a C++ high-frequency trading (HFT) project built with CMake. The project is in early development phase with a
focus on low-latency trading systems and object pooling for performance optimization.

# common

用中文

## Build and Development Commands

### Building the Project

```bash
# Configure and build using CMake
mkdir -p cmake-build-debug
cd cmake-build-debug
cmake ..
make
```

### Running the Application

```bash
# From build directory
./hft

# Or from root directory
./cmake-build-debug/hft
```

### Development Workflow

```bash
# Clean build
rm -rf cmake-build-debug
mkdir cmake-build-debug
cd cmake-build-debug
cmake ..
make
```

## Architecture and Code Structure

### Core Components

- **main.cpp**: Entry point containing basic Hello World example and debugging setup
- **CMakeLists.txt**: Build configuration using C++20 standard
- **Object Pool System**: Planned high-priority feature for memory-efficient object management

### Development Guidelines

- Uses C++20 standard
- Focus on low-latency performance optimizations
- Memory management through object pooling patterns
- Test-driven development with examples for each feature

## Custom Commands

### /exp - Feature Implementation Command

Implements user stories defined in `.claude/story/user_story_example.yaml`.

Usage:

```bash
/exp [story_id]    # Implement specific story (e.g., US-001)
/exp               # Implement all stories
```

Implementation approach:

1. Quick prototype - minimal working version
2. Core functionality - main feature modules
3. Real testing - validate with actual data
4. Optimization - performance improvements
5. Documentation - usage examples

### Current User Stories

- **US-001**: Object Pool Implementation
    - Priority: High
    - Goal: Memory-efficient object management without malloc overhead
    - Requirements: No memory leaks, low latency
    - Components: object generation, retrieval, return mechanisms

## Development Focus

This project prioritizes:

1. **Performance**: Low-latency optimizations for trading systems
2. **Memory Efficiency**: Object pooling to avoid frequent allocations
3. **Real-world Testing**: Validation with actual trading scenarios
4. **Iterative Development**: Working prototypes first, then optimization