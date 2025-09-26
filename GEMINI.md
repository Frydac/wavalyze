# Wavalyze Project Overview

This document provides a high-level overview of the Wavalyze project, intended to help contributors understand its purpose, structure, and key technologies.

## Purpose

Wavalyze is a waveform analysis tool built in Rust. It allows users to load, visualize, and analyze audio files in WAV format. The application provides a graphical user interface to inspect audio waveforms, view properties, and potentially apply various analysis functions. It is designed to be compiled for both native desktop and web (WASM) environments.

## Key Technologies

*   **Rust**: The core language for the application.
*   **egui/eframe**: A simple, fast, and highly portable immediate mode GUI library in Rust. It is used for all UI components.
*   **hound**: A crate for reading and writing WAV audio files.
*   **Trunk**: Used for building and bundling the application for the web (WASM).
*   **Clap**: For command-line argument parsing.

## Project Structure

The project is structured as a standard Rust binary crate.

*   `src/main.rs`: The entry point of the application. It handles command-line argument parsing and initializes the `eframe` application.
*   `src/app.rs`: Contains the main application state and the top-level UI structure, managed by the `eframe` framework.
*   `src/lib.rs`: Crate-level library definitions.
*   `src/audio/`: Contains modules related to audio processing, such as buffer management, sample manipulation, and analysis functions (e.g., RMS, cross-correlation).
*   `src/model/`: Defines the core data structures of the application, such as tracks, buffers, and application configuration.
*   `src/view/`: Contains the UI components for different parts of the application, like the tracks, ruler, and configuration panels.
*   `src/wav/`: Handles reading and parsing of WAV files.
*   `tests/`: Contains integration tests.
*   `assets/`: Static assets for the web build, such as icons and the service worker.
*   `Cargo.toml`: The Rust package manager configuration file, defining dependencies and project metadata.
*   `Trunk.toml`: Configuration for the Trunk build tool.
