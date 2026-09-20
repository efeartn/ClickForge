# Contributing to ClickForge

First off, thank you for considering contributing to ClickForge! It's people like you that make open-source a great community.

## Development Setup

1. **Fork and Clone**: Fork the repository on GitHub and clone it locally.
   ```bash
   git clone https://github.com/efeartn/ClickForge.git
   cd ClickForge
   ```
2. **Create a Virtual Environment**:
   ```bash
   python -m venv venv
   # On Windows
   venv\Scripts\activate
   # On macOS/Linux
   source venv/bin/activate
   ```
3. **Install Dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

## Code Style

- We follow [PEP 8](https://peps.python.org/pep-0008/) style guidelines.
- **Type Hints**: Please use type hints for function arguments and return values where possible.
- **Docstrings**: Ensure all public modules, classes, and methods have descriptive docstrings (we prefer Google or Sphinx style).

## Pull Request Process

1. **Branch**: Create a new branch for your feature or bug fix (`git checkout -b feature/amazing-feature`).
2. **Develop**: Write your code.
3. **Test**: Run existing tests and add new ones if applicable. Make sure everything works as expected.
4. **Commit**: Commit your changes with descriptive commit messages.
5. **Push**: Push to your fork (`git push origin feature/amazing-feature`).
6. **PR**: Open a Pull Request against the `main` branch of the upstream repository. Provide a clear description of your changes.

## Issue Reporting

If you find a bug or have a feature request, please open an issue and include:
- A clear, descriptive title.
- Steps to reproduce (for bugs).
- Expected vs. actual behavior.
- Your OS and Python version.

## Code of Conduct

Please note that this project is released with a Contributor Code of Conduct. By participating in this project you agree to abide by its terms. Let's keep the community welcoming and respectful!
