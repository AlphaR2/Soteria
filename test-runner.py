#!/usr/bin/env python3
"""
Soteria Security Test Runner (Python)
Interactive script to build and test all Solana security programs

Compatible with: Linux, macOS, Windows (WSL recommended)
"""

import os
import subprocess
import sys
from typing import Optional, List, Tuple

# ANSI color codes
class Colors:
    RESET = '\033[0m'
    BOLD = '\033[1m'
    DIM = '\033[2m'

    # Foreground colors
    RED = '\033[0;31m'
    GREEN = '\033[0;32m'
    YELLOW = '\033[1;33m'
    BLUE = '\033[0;34m'
    MAGENTA = '\033[0;35m'
    CYAN = '\033[0;36m'
    WHITE = '\033[1;37m'

class UI:
    """UI helper functions"""

    @staticmethod
    def clear():
        os.system('cls' if os.name == 'nt' else 'clear')

    @staticmethod
    def header(text: str = "Soteria Security Test Runner"):
        """Print header box"""
        width = 70
        print(f"\n{Colors.CYAN}{'═' * width}{Colors.RESET}")
        print(f"{Colors.CYAN}{Colors.BOLD}{text:^{width}}{Colors.RESET}")
        print(f"{Colors.CYAN}{'═' * width}{Colors.RESET}\n")

    @staticmethod
    def section(text: str):
        """Print section header"""
        print(f"\n{Colors.YELLOW}{'▶'} {Colors.BOLD}{text}{Colors.RESET}")
        print(f"{Colors.BLUE}{'─' * 60}{Colors.RESET}\n")

    @staticmethod
    def success(text: str):
        print(f"{Colors.GREEN}✓ {text}{Colors.RESET}")

    @staticmethod
    def error(text: str):
        print(f"{Colors.RED}✗ {text}{Colors.RESET}")

    @staticmethod
    def warning(text: str):
        print(f"{Colors.YELLOW}⚠ {text}{Colors.RESET}")

    @staticmethod
    def info(text: str):
        print(f"{Colors.BLUE}ℹ {text}{Colors.RESET}")

    @staticmethod
    def progress(current: int, total: int):
        """Show progress bar"""
        percent = int((current / total) * 100)
        filled = int((current / total) * 40)
        bar = '█' * filled + '░' * (40 - filled)
        print(f"\r{Colors.CYAN}Progress: [{bar}] {percent}% ({current}/{total}){Colors.RESET}", end='', flush=True)
        if current == total:
            print()  # New line when complete

def run_command(cmd: List[str], cwd: Optional[str] = None, capture: bool = False) -> Tuple[bool, str]:
    """Run a shell command"""
    try:
        if capture:
            result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
            return result.returncode == 0, result.stdout
        else:
            result = subprocess.run(cmd, cwd=cwd)
            return result.returncode == 0, ""
    except Exception as e:
        UI.error(f"Command failed: {e}")
        return False, ""

def build_program(program: str, version: str) -> bool:
    """Build a specific program"""
    UI.section(f"Building {program}/{version}")

    path = os.path.join("programs", program, version)

    # Determine build command based on program type
    if program == "pino-escrow":
        cmd = ["cargo", "build"]
    else:
        cmd = ["cargo", "build-sbf"]

    UI.info(f"Building in {path}...")
    success, _ = run_command(cmd, cwd=path)

    if success:
        UI.success(f"Built {program}/{version}")
    else:
        UI.error(f"Failed to build {program}/{version}")

    return success

def run_tests(program: str, version: str, test_name: Optional[str] = None) -> bool:
    """Run tests for a specific program"""
    path = os.path.join("programs", program, version)

    # Determine test command based on program type
    if program == "pino-escrow":
        base_cmd = ["cargo", "test"]
    else:
        base_cmd = ["cargo", "test-sbf"]

    if test_name:
        cmd = base_cmd + [test_name, "--", "--nocapture"]
        UI.section(f"Running test: {test_name}")
    else:
        cmd = base_cmd + ["--", "--nocapture"]
        UI.section(f"Running all tests for {program}/{version}")

    UI.info(f"Executing tests in {path}...")
    success, _ = run_command(cmd, cwd=path)

    if success:
        UI.success("Tests passed")
    else:
        UI.error("Tests failed")

    return success

def show_main_menu():
    """Display main menu"""
    UI.clear()
    UI.header()

    print(f"{Colors.BOLD}Main Menu{Colors.RESET}\n")
    print(f"  {Colors.CYAN}1{Colors.RESET} │ {Colors.BOLD}Multisig{Colors.RESET}                 {Colors.DIM}Multi-signature wallet tests{Colors.RESET}")
    print(f"  {Colors.CYAN}2{Colors.RESET} │ {Colors.BOLD}Governance{Colors.RESET}               {Colors.DIM}Reputation-based DAO tests{Colors.RESET}")
    print(f"  {Colors.CYAN}3{Colors.RESET} │ {Colors.BOLD}AMM{Colors.RESET}                      {Colors.DIM}Automated Market Maker tests{Colors.RESET}")
    print(f"  {Colors.CYAN}4{Colors.RESET} │ {Colors.BOLD}Escrow (Pinocchio){Colors.RESET}       {Colors.DIM}Atomic swap escrow tests{Colors.RESET}")
    print(f"  {Colors.CYAN}5{Colors.RESET} │ {Colors.BOLD}NFT Minting{Colors.RESET}              {Colors.DIM}NFT minting & Metaplex tests{Colors.RESET}")
    print(f"  {Colors.BLUE}──┼{'─' * 60}{Colors.RESET}")
    print(f"  {Colors.CYAN}6{Colors.RESET} │ {Colors.BOLD}Run All Tests{Colors.RESET}            {Colors.DIM}Execute all test suites{Colors.RESET}")
    print(f"  {Colors.CYAN}7{Colors.RESET} │ {Colors.BOLD}Build Programs{Colors.RESET}           {Colors.DIM}Compile Solana programs{Colors.RESET}")
    print(f"  {Colors.BLUE}──┼{'─' * 60}{Colors.RESET}")
    print(f"  {Colors.RED}0{Colors.RESET} │ {Colors.BOLD}Exit{Colors.RESET}")
    print()

def show_program_menu(program_name: str, secure_count: str, exploit_count: str):
    """Display program test menu"""
    UI.clear()
    UI.header(f"{program_name} Tests")

    print(f"{Colors.BOLD}Select Test Type{Colors.RESET}\n")
    print(f"  {Colors.GREEN}1{Colors.RESET} │ {Colors.BOLD}Run Secure Tests{Colors.RESET}         {Colors.DIM}({secure_count} tests){Colors.RESET}")
    print(f"  {Colors.RED}2{Colors.RESET} │ {Colors.BOLD}Run Exploit Tests{Colors.RESET}        {Colors.DIM}({exploit_count} tests){Colors.RESET}")
    print(f"  {Colors.CYAN}3{Colors.RESET} │ {Colors.BOLD}Run Both{Colors.RESET}                 {Colors.DIM}({int(secure_count) + int(exploit_count) if secure_count.isdigit() and exploit_count.isdigit() else 'TBD'} tests total){Colors.RESET}")
    print(f"  {Colors.BLUE}──┼{'─' * 60}{Colors.RESET}")
    print(f"  {Colors.YELLOW}0{Colors.RESET} │ {Colors.BOLD}Back to Main Menu{Colors.RESET}")
    print()

def show_build_menu():
    """Display build menu"""
    UI.clear()
    UI.header("Build Programs")

    print(f"{Colors.BOLD}Select Program to Build{Colors.RESET}\n")
    print(f"  {Colors.GREEN}1 {Colors.RESET} │ Multisig     {Colors.BOLD}Secure{Colors.RESET}      {Colors.DIM}programs/multisig/m-secure{Colors.RESET}")
    print(f"  {Colors.RED}2 {Colors.RESET} │ Multisig     {Colors.BOLD}Vulnerable{Colors.RESET}  {Colors.DIM}programs/multisig/m-vulnerable{Colors.RESET}")
    print(f"  {Colors.GREEN}3 {Colors.RESET} │ Governance   {Colors.BOLD}Secure{Colors.RESET}      {Colors.DIM}programs/governance/g-secure{Colors.RESET}")
    print(f"  {Colors.RED}4 {Colors.RESET} │ Governance   {Colors.BOLD}Vulnerable{Colors.RESET}  {Colors.DIM}programs/governance/g-vulnerable{Colors.RESET}")
    print(f"  {Colors.GREEN}5 {Colors.RESET} │ AMM          {Colors.BOLD}Secure{Colors.RESET}      {Colors.DIM}programs/amm/amm-secure{Colors.RESET}")
    print(f"  {Colors.RED}6 {Colors.RESET} │ AMM          {Colors.BOLD}Vulnerable{Colors.RESET}  {Colors.DIM}programs/amm/amm-vulnerable{Colors.RESET}")
    print(f"  {Colors.GREEN}7 {Colors.RESET} │ Escrow       {Colors.BOLD}Secure{Colors.RESET}      {Colors.DIM}programs/pino-escrow/p-secure{Colors.RESET}")
    print(f"  {Colors.RED}8 {Colors.RESET} │ Escrow       {Colors.BOLD}Vulnerable{Colors.RESET}  {Colors.DIM}programs/pino-escrow/p-vulnerable{Colors.RESET}")
    print(f"  {Colors.GREEN}9 {Colors.RESET} │ NFT Minting  {Colors.BOLD}Secure{Colors.RESET}      {Colors.DIM}programs/nfts/n-secure{Colors.RESET}")
    print(f"  {Colors.RED}10{Colors.RESET} │ NFT Minting  {Colors.BOLD}Vulnerable{Colors.RESET}  {Colors.DIM}programs/nfts/n-vulnerable{Colors.RESET}")
    print(f"  {Colors.BLUE}──┼{'─' * 60}{Colors.RESET}")
    print(f"  {Colors.MAGENTA}A {Colors.RESET} │ {Colors.BOLD}Build All Programs{Colors.RESET}  {Colors.DIM}Sequential build (10 programs){Colors.RESET}")
    print(f"  {Colors.BLUE}──┼{'─' * 60}{Colors.RESET}")
    print(f"  {Colors.YELLOW}0 {Colors.RESET} │ {Colors.BOLD}Back to Main Menu{Colors.RESET}")
    print()

def handle_program_tests(program: str, secure_ver: str, vuln_ver: str,
                         name: str, secure_count: str, exploit_count: str):
    """Handle tests for a specific program"""
    while True:
        show_program_menu(name, secure_count, exploit_count)
        choice = input(f"{Colors.BOLD}Choose an option:{Colors.RESET} ").strip()

        if choice == '0':
            break
        elif choice == '1':
            run_tests(program, secure_ver)
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")
        elif choice == '2':
            run_tests(program, vuln_ver)
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")
        elif choice == '3':
            run_tests(program, secure_ver)
            run_tests(program, vuln_ver)
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")
        else:
            UI.error("Invalid choice")
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")

def handle_build():
    """Handle build menu"""
    programs = [
        ("multisig", "m-secure"),
        ("multisig", "m-vulnerable"),
        ("governance", "g-secure"),
        ("governance", "g-vulnerable"),
        ("amm", "amm-secure"),
        ("amm", "amm-vulnerable"),
        ("pino-escrow", "p-secure"),
        ("pino-escrow", "p-vulnerable"),
        ("nfts", "n-secure"),
        ("nfts", "n-vulnerable"),
    ]

    while True:
        show_build_menu()
        choice = input(f"{Colors.BOLD}Choose an option:{Colors.RESET} ").strip()

        if choice == '0':
            break
        elif choice in ['1', '2', '3', '4', '5', '6', '7', '8', '9', '10']:
            idx = int(choice) - 1
            program, version = programs[idx]
            build_program(program, version)
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")
        elif choice.upper() == 'A':
            UI.section("Building All Programs")
            total = len(programs)
            for i, (program, version) in enumerate(programs, 1):
                UI.progress(i - 1, total)
                if not build_program(program, version):
                    UI.error("Build sequence interrupted")
                    break
                UI.progress(i, total)
            else:
                print()
                UI.success("All programs built successfully!")
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")
        else:
            UI.error("Invalid choice")
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")

def run_all():
    """Run all tests"""
    UI.section("Running All Tests")

    tests = [
        ("multisig", "m-secure", "Multisig Secure"),
        ("multisig", "m-vulnerable", "Multisig Vulnerable"),
        ("governance", "g-secure", "Governance Secure"),
        ("governance", "g-vulnerable", "Governance Vulnerable"),
        ("amm", "amm-secure", "AMM Secure"),
        ("amm", "amm-vulnerable", "AMM Vulnerable"),
        ("pino-escrow", "p-secure", "Escrow Secure"),
        ("pino-escrow", "p-vulnerable", "Escrow Vulnerable"),
        ("nfts", "n-secure", "NFT Secure"),
        ("nfts", "n-vulnerable", "NFT Vulnerable"),
    ]

    total = len(tests)
    for i, (program, version, name) in enumerate(tests, 1):
        UI.progress(i - 1, total)
        print(f"\n{Colors.CYAN}Testing {name}...{Colors.RESET}")
        run_tests(program, version)
        UI.progress(i, total)

    print()
    UI.success("All test suites completed!")
    input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")

def main():
    """Main entry point"""
    # Check if we're in the right directory
    if not os.path.isdir("programs"):
        UI.clear()
        UI.header()
        UI.error("Must be run from the Soteria project root directory")
        UI.info("Please cd to the project root and try again")
        sys.exit(1)

    # Program configurations: (folder, secure_ver, vuln_ver, name, secure_tests, exploit_tests)
    program_configs = [
        ("multisig", "m-secure", "m-vulnerable", "Multisig", "4", "4"),
        ("governance", "g-secure", "g-vulnerable", "Governance", "5", "6"),
        ("amm", "amm-secure", "amm-vulnerable", "AMM", "5", "7"),
        ("pino-escrow", "p-secure", "p-vulnerable", "Pino Escrow", "TBD", "TBD"),
        ("nfts", "n-secure", "n-vulnerable", "NFT Minting", "TBD", "TBD"),
    ]

    while True:
        show_main_menu()
        choice = input(f"{Colors.BOLD}Choose an option:{Colors.RESET} ").strip()

        if choice == '0':
            UI.clear()
            print()
            print(f"{Colors.CYAN}{'═' * 50}{Colors.RESET}")
            print(f"{Colors.CYAN}{Colors.BOLD}{'Thank you for using Soteria!':^50}{Colors.RESET}")
            print(f"{Colors.CYAN}{'═' * 50}{Colors.RESET}")
            print()
            print(f"{Colors.DIM}Stay secure. Happy hacking!{Colors.RESET}\n")
            break
        elif choice in ['1', '2', '3', '4', '5']:
            idx = int(choice) - 1
            handle_program_tests(*program_configs[idx])
        elif choice == '6':
            run_all()
        elif choice == '7':
            handle_build()
        else:
            UI.error("Invalid choice")
            input(f"\n{Colors.DIM}Press Enter to continue...{Colors.RESET}")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n\n{Colors.YELLOW}⚠ Interrupted by user{Colors.RESET}\n")
        sys.exit(0)
    except Exception as e:
        UI.error(f"Unexpected error: {e}")
        sys.exit(1)
