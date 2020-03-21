# Racemus
[![Coverage Status](https://coveralls.io/repos/github/jcdickinson/racemus/badge.svg)](https://coveralls.io/github/jcdickinson/racemus)

Racemus is an experimental Minecraft server.

# Tasks

- [x] Login Sequence
- [x] Tests for login sequence 50%
- [x] Start simulation
- [x] Server status - basic
- [ ] 1 chunk flatland

# Getting Started

1. `generate-key.ps1` is both a Powershell and Bash script. You will need
   openssl on the path.
2. `rustup target add x86_64-pc-windows-gnu --toolchain nightly`
3. Once you have a key, cargo run should just work.
