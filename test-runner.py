#!/usr/bin/env python3
"""
Soteria Security Test Runner
Interactive Python script to run security tests for all programs
"""

import os
import subprocess
import sys
from typing import Optional

# ANSI color codes
class Colors:
    RED = '\033[0;31m'
    GREEN = '\033[0;32m'
    YELLOW = '\033[1;33m'
    BLUE = '\033[0;34m'
    CYAN = '\033[0;36m'
    NC = '\033[0m'  # No Color

def print_header(text: str):
    print(f"\n{Colors.CYAN}{'=' * 40}{Colors.NC}")
    print(f"{Colors.CYAN}{text}{Colors.NC}")
    print(f"{Colors.CYAN}{'=' * 40}{Colors.NC}\n")

def print_success(text: str):
    print(f"{Colors.GREEN}[SUCCESS] {text}{Colors.NC}")

def print_error(text: str):
    print(f"{Colors.RED}[ERROR] {text}{Colors.NC}")

def print_warning(text: str):
    print(f"{Colors.YELLOW}[WARNING] {text}{Colors.NC}")

def print_info(text: str):
    print(f"{Colors.BLUE}[INFO] {text}{Colors.NC}")

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def run_command(cmd: list, cwd: Optional[str] = None) -> bool:
    """Run a shell command and return success status"""
    try:
        result = subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=False,
            text=True
        )
        return result.returncode == 0
    except Exception as e:
        print_error(f"Error running command: {e}")
        return False

def build_program(program: str, version: str) -> bool:
    """Build a specific program"""
    print_header(f"Building {program} ({version})")

    path = os.path.join("programs", program, version)
    print_info("Building program...")

    success = run_command(["cargo", "build-sbf"], cwd=path)

    if success:
        print_success("Build completed successfully")
    else:
        print_error("Build failed")

    return success

def run_tests(program: str, version: str, test_name: Optional[str] = None):
    """Run tests for a specific program"""
    if test_name:
        print_header(f"Running {test_name} test for {program} ({version})")
    else:
        print_header(f"Running all tests for {program} ({version})")

    path = os.path.join("programs", program, version)

    if test_name:
        cmd = ["cargo", "test", test_name, "--", "--nocapture"]
    else:
        cmd = ["cargo", "test", "--", "--nocapture"]

    print_info("Running tests...")
    success = run_command(cmd, cwd=path)

    if success:
        print_success("Tests completed successfully")
    else:
        print_error("Tests failed")

    input("\nPress Enter to continue...")

def list_tests(program: str, version: str) -> list:
    """List available tests for a program"""
    path = os.path.join("programs", program, version)

    try:
        result = subprocess.run(
            ["cargo", "test", "--", "--list"],
            cwd=path,
            capture_output=True,
            text=True
        )

        tests = []
        for line in result.stdout.split('\n'):
            line = line.strip()
            if line.startswith('test_') or line.startswith('exploit_'):
                test_name = line.split(':')[0]
                tests.append(test_name)

        return tests
    except Exception:
        return []

def show_main_menu():
    """Display main menu"""
    clear_screen()
    print_header("Soteria Security Test Runner")

    print(f"{Colors.CYAN}Select a program to test:{Colors.NC}")
    print("1. Pino Escrow")
    print("2. Multisig")
    print("3. NFTs (Staking)")
    print("4. Run all programs")
    print("5. Build all programs")
    print("0. Exit")
    print()

def show_version_menu(program_name: str):
    """Display version selection menu"""
    clear_screen()
    print_header(f"{program_name} Tests")

    print(f"{Colors.CYAN}Select version:{Colors.NC}")
    print("1. Secure version")
    print("2. Vulnerable version")
    print("3. Both versions")
    print("0. Back to main menu")
    print()

def show_test_type_menu():
    """Display test type menu"""
    clear_screen()
    print_header("Test Execution Options")

    print(f"{Colors.CYAN}How would you like to run the tests?{Colors.NC}")
    print("1. Run all tests")
    print("2. Run specific test")
    print("0. Back")
    print()

def handle_program_tests(program: str, program_name: str):
    """Handle test execution for a specific program"""
    while True:
        show_version_menu(program_name)
        choice = input("Enter choice: ").strip()

        if choice == '0':
            break
        elif choice in ['1', '2', '3']:
            versions = []
            # Determine prefix based on program
            prefix = program.split('-')[0][0]  # Get first letter: p, m, or n
            if choice == '1':
                versions = [f'{prefix}-secure']
            elif choice == '2':
                versions = [f'{prefix}-vulnerable']
            else:
                versions = [f'{prefix}-secure', f'{prefix}-vulnerable']

            for version in versions:
                show_test_type_menu()
                test_choice = input("Enter choice: ").strip()

                if test_choice == '0':
                    continue
                elif test_choice == '1':
                    run_tests(program, version)
                elif test_choice == '2':
                    # List available tests
                    tests = list_tests(program, version)
                    if tests:
                        print(f"\n{Colors.YELLOW}Available tests:{Colors.NC}")
                        for i, test in enumerate(tests, 1):
                            print(f"{i}. {test}")
                        print()

                        test_idx = input("Enter test number (or 'all' for all tests): ").strip()

                        if test_idx.lower() == 'all':
                            run_tests(program, version)
                        elif test_idx.isdigit() and 1 <= int(test_idx) <= len(tests):
                            run_tests(program, version, tests[int(test_idx) - 1])
                        else:
                            print_error("Invalid choice")
                            input("Press Enter to continue...")
                    else:
                        print_warning("No tests found")
                        input("Press Enter to continue...")
        else:
            print_error("Invalid choice")
            input("Press Enter to continue...")

def build_all():
    """Build all programs"""
    print_header("Building All Programs")

    programs = [
        ("pino-escrow", "p-secure"),
        ("pino-escrow", "p-vulnerable"),
        ("multisig", "m-secure"),
        ("multisig", "m-vulnerable"),
        ("nfts", "n-secure"),
        ("nfts", "n-vulnerable"),
    ]

    for program, version in programs:
        if not build_program(program, version):
            print_error(f"Failed to build {program} ({version})")
            input("\nPress Enter to continue...")
            return

    print_success("All programs built successfully!")
    input("\nPress Enter to continue...")

def run_all():
    """Run all tests"""
    print_header("Running All Tests")

    programs = [
        ("pino-escrow", "p-secure", "Escrow - Secure"),
        ("pino-escrow", "p-vulnerable", "Escrow - Vulnerable"),
        ("multisig", "m-secure", "Multisig - Secure"),
        ("multisig", "m-vulnerable", "Multisig - Vulnerable"),
        ("nfts", "n-secure", "NFTs - Secure"),
        ("nfts", "n-vulnerable", "NFTs - Vulnerable"),
    ]

    for program, version, label in programs:
        print_info(f"{label} Tests")
        run_tests(program, version)

    print_success("All tests completed!")
    input("\nPress Enter to continue...")

def main():
    """Main entry point"""
    # Check if we're in the right directory
    if not os.path.isdir("programs"):
        print_error("Error: Must be run from the Soteria project root directory")
        sys.exit(1)

    while True:
        show_main_menu()
        choice = input("Enter choice: ").strip()

        if choice == '0':
            print_success("Goodbye!")
            break
        elif choice == '1':
            handle_program_tests("pino-escrow", "Pino Escrow")
        elif choice == '2':
            handle_program_tests("multisig", "Multisig")
        elif choice == '3':
            handle_program_tests("nfts", "NFT Staking")
        elif choice == '4':
            run_all()
        elif choice == '5':
            build_all()
        else:
            print_error("Invalid choice")
            input("Press Enter to continue...")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n\n{Colors.YELLOW}Interrupted by user{Colors.NC}")
        sys.exit(0)
