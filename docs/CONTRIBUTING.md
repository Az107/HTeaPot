# Contributing to HTeaPot

Thanks for your interest in contributing to HTeaPot!
This project is meant to be minimal, clean, and dependency-free.

## 🧾 General guidelines

- Keep the project as dependency-free as possible.
- Prioritize simplicity and readability over cleverness.
- This project aims to stay **dependency-free** (within reason).
- **Any pull request that adds an external crate will be rejected** unless it's absolutely necessary and thoroughly justified.
- If you think a dependency is truly essential, open an issue first and explain the rational
- Avoid pulling in large changes without discussion first (open an issue or start a draft PR).

## 📐 Code style

- Use idiomatic Rust whenever possible.
- Keep formatting consistent with `rustfmt`.
- Write comments where it helps understanding, especially for lower-level code or unsafe blocks.

## 🚀 Getting started

1. Clone the repo:
   ```bash
   git clone https://github.com/tuusuario/hteapot.git
   cd hteapot
   ```


2. Build
   ```bash
   cargo build
   ```

## 🛠️ How to contribute

- Open an issue to suggest features or report bugs.
- Fork the repo and create a branch from main.
- Submit a pull request when ready. It’s okay if it’s not perfect — we can improve it together.

## Philosophy

HTeaPot is a minimalist HTTP server, and the idea is to stay light and focused.
Please avoid turning it into a framework
