#!/bin/bash
# Soteria Test Runner - Enhanced Interactive UI
# Run security tests and build programs with style
set -e

# Enhanced color palette
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# Box drawing characters
BOX_H="━"
BOX_V="┃"
BOX_TL="┏"
BOX_TR="┓"
BOX_BL="┗"
BOX_BR="┛"
BOX_VR="┣"
BOX_VL="┫"
BOX_HT="┳"
BOX_HB="┻"
BOX_CROSS="╋"

# Function to draw a fancy box
draw_box() {
    local title="$1"
    local width=60
    local padding=$(( (width - ${#title} - 2) / 2 ))

    echo -e "${CYAN}${BOX_TL}$(printf '%*s' $width | tr ' ' "$BOX_H")${BOX_TR}${NC}"
    printf "${CYAN}${BOX_V}${NC}%*s${BOLD}%s${NC}%*s${CYAN}${BOX_V}${NC}\n" $padding "" "$title" $((width - padding - ${#title})) ""
    echo -e "${CYAN}${BOX_BL}$(printf '%*s' $width | tr ' ' "$BOX_H")${BOX_BR}${NC}"
}

# Print fancy header with gradient effect
print_header() {
    clear
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${BOLD}        SOTERIA TEST SUITE                ${NC}${CYAN}║${NC}"
    echo -e "${CYAN}╠════════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} ${DIM}Solana Program Vulnerability Testing ${NC}    ${CYAN}║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    if [ -n "$1" ]; then
        echo -e "${MAGENTA}▶ $1${NC}"
        echo ""
    fi
}

# Section header
print_section() {
    echo ""
    echo -e "${BLUE}┌─────────────────────────────────────────────────────────────┐${NC}"
    echo -e "${BLUE}│${NC} ${BOLD}$1${NC}"
    echo -e "${BLUE}└─────────────────────────────────────────────────────────────┘${NC}"
}

# Status messages with icons
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}${NC} $1"
}

print_building() {
    echo -e "${CYAN}${NC} $1"
}

print_testing() {
    echo -e "${MAGENTA}${NC} $1"
}

# Progress indicator
show_progress() {
    local current=$1
    local total=$2
    local percent=$((current * 100 / total))
    local filled=$((percent / 5))
    local empty=$((20 - filled))

    printf "\r${CYAN}["
    printf "%${filled}s" | tr ' ' '█'
    printf "%${empty}s" | tr ' ' '░'
    printf "]${NC} ${BOLD}%3d%%${NC} (%d/%d)" $percent $current $total
}

# Build a single program with enhanced output
build_program() {
    local program=$1
    local version=$2
    local dir="programs/$program/$version"

    print_section "Building $program ($version)"

    if [ ! -d "$dir" ]; then
        print_error "Directory not found: $dir"
        return 1
    fi

    cd "$dir" || return 1
    print_building "Compiling with cargo build-sbf..."
    print_info "Target directory: $dir/target"

    echo ""
    if cargo build-sbf -- --target-dir ./target 2>&1 | while IFS= read -r line; do
        if echo "$line" | grep -q "Finished"; then
            echo -e "${GREEN}$line${NC}"
        elif echo "$line" | grep -q "Compiling"; then
            echo -e "${CYAN}$line${NC}"
        elif echo "$line" | grep -q "error"; then
            echo -e "${RED}$line${NC}"
        elif echo "$line" | grep -q "warning"; then
            echo -e "${YELLOW}$line${NC}"
        else
            echo "$line"
        fi
    done; then
        echo ""
        print_success "Build completed successfully!"
        print_info "Binary: $dir/target/deploy/"
    else
        echo ""
        print_error "Build failed - check output above"
        cd - > /dev/null
        return 1
    fi

    cd - > /dev/null
    echo ""
}

# Run tests with enhanced output
run_program_tests() {
    local program=$1
    local version=$2
    local test_type=$3

    print_section "$test_type tests: $program ($version)"

    cd "programs/$program/$version" || {
        print_error "Directory not found: programs/$program/$version"
        return 1
    }

    if [ "$test_type" == "ALL" ]; then
        print_testing "Running all tests..."
        echo ""
        cargo test -- --nocapture 2>&1 | while IFS= read -r line; do
            if echo "$line" | grep -q "test result:"; then
                echo -e "${BOLD}$line${NC}"
            elif echo "$line" | grep -q "PASSED\|ok"; then
                echo -e "${GREEN}$line${NC}"
            elif echo "$line" | grep -q "FAILED\|FAIL"; then
                echo -e "${RED}$line${NC}"
            elif echo "$line" | grep -q "running"; then
                echo -e "${CYAN}$line${NC}"
            else
                echo "$line"
            fi
        done
        local result=$?
    elif [ "$test_type" == "SPECIFIC" ]; then
        echo -e "${YELLOW}┌─ Available Tests ─────────────────────────────────────────┐${NC}"
        cargo test -- --list 2>/dev/null | grep -E "^test_|^exploit_" | nl -w2 -s". " | while IFS= read -r line; do
            echo -e "  ${CYAN}$line${NC}"
        done
        echo -e "${YELLOW}└───────────────────────────────────────────────────────────┘${NC}"
        echo ""
        echo -e "${BOLD}Enter test name${NC} ${DIM}(or 'all' for all tests):${NC} "
        read test_name
        echo ""
        if [ "$test_name" == "all" ]; then
            cargo test -- --nocapture
        else
            cargo test "$test_name" -- --nocapture
        fi
        local result=$?
    fi

    echo ""
    if [ $result -eq 0 ]; then
        print_success "Tests completed successfully"
    else
        print_error "Some tests failed"
    fi

    cd - > /dev/null
}

# Main menu with enhanced UI
show_main_menu() {
    print_header

    echo -e "${BOLD}Main Menu${NC}"
    echo ""
    echo -e "  ${CYAN}1${NC} │ ${BOLD}Escrow (Pinocchio)${NC}       ${DIM}Cross-program invocation tests${NC}"
    echo -e "  ${CYAN}2${NC} │ ${BOLD}Multisig${NC}                 ${DIM}Multi-signature wallet tests${NC}"
    echo -e "  ${CYAN}3${NC} │ ${BOLD}NFT Staking${NC}              ${DIM}NFT staking & rewards tests${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${CYAN}4${NC} │ ${BOLD}Run All Tests${NC}            ${DIM}Execute all test suites${NC}"
    echo -e "  ${CYAN}5${NC} │ ${BOLD}Build Programs${NC}           ${DIM}Compile Solana programs${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${RED}0${NC} │ ${BOLD}Exit${NC}"
    echo ""
    echo -ne "${BOLD}Choose an option:${NC} "
}

# Program-specific menus
show_program_menu() {
    local program_name=$1
    local secure_count=$2
    local vuln_count=$3

    print_header "$program_name Test Suite"

    echo -e "${BOLD}Select Version${NC}"
    echo ""
    echo -e "  ${GREEN}1${NC} │ ${BOLD}Secure Version${NC}      ${DIM}($secure_count tests)${NC}"
    echo -e "  ${RED}2${NC} │ ${BOLD}Vulnerable Version${NC}  ${DIM}($vuln_count exploit tests)${NC}"
    echo -e "  ${CYAN}3${NC} │ ${BOLD}Both Versions${NC}       ${DIM}Run secure + vulnerable${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${YELLOW}0${NC} │ ${BOLD}Back to Main Menu${NC}"
    echo ""
    echo -ne "${BOLD}Choose an option:${NC} "
}

# Test type menu
show_test_type_menu() {
    print_header "Test Execution Mode"

    echo -e "${BOLD}How would you like to run the tests?${NC}"
    echo ""
    echo -e "  ${CYAN}1${NC} │ ${BOLD}All Tests${NC}           ${DIM}Run complete test suite${NC}"
    echo -e "  ${CYAN}2${NC} │ ${BOLD}Specific Test${NC}       ${DIM}Select individual test${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${YELLOW}0${NC} │ ${BOLD}Back${NC}"
    echo ""
    echo -ne "${BOLD}Choose an option:${NC} "
}

# Build menu with progress
show_build_menu() {
    print_header "Build Programs"

    echo -e "${BOLD}Select Program to Build${NC}"
    echo ""
    echo -e "  ${GREEN}1${NC} │ Escrow       ${BOLD}Secure${NC}      ${DIM}programs/pino-escrow/p-secure${NC}"
    echo -e "  ${RED}2${NC} │ Escrow       ${BOLD}Vulnerable${NC}  ${DIM}programs/pino-escrow/p-vulnerable${NC}"
    echo -e "  ${GREEN}3${NC} │ Multisig     ${BOLD}Secure${NC}      ${DIM}programs/multisig/m-secure${NC}"
    echo -e "  ${RED}4${NC} │ Multisig     ${BOLD}Vulnerable${NC}  ${DIM}programs/multisig/m-vulnerable${NC}"
    echo -e "  ${GREEN}5${NC} │ NFT Staking  ${BOLD}Secure${NC}      ${DIM}programs/nfts/n-secure${NC}"
    echo -e "  ${RED}6${NC} │ NFT Staking  ${BOLD}Vulnerable${NC}  ${DIM}programs/nfts/n-vulnerable${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${MAGENTA}A${NC} │ ${BOLD}Build All Programs${NC}  ${DIM}Sequential build (6 programs)${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${YELLOW}0${NC} │ ${BOLD}Back to Main Menu${NC}"
    echo ""
    echo -ne "${BOLD}Choose an option:${NC} "
}

# Handle build menu
handle_build() {
    while true; do
        show_build_menu
        read build_choice
        echo ""

        case $build_choice in
            1) build_program "pino-escrow" "p-secure";;
            2) build_program "pino-escrow" "p-vulnerable";;
            3) build_program "multisig" "m-secure";;
            4) build_program "multisig" "m-vulnerable";;
            5) build_program "nfts" "n-secure";;
            6) build_program "nfts" "n-vulnerable";;
            a|A)
                print_section "Building All Programs"
                echo ""
                local programs=("pino-escrow:p-secure" "pino-escrow:p-vulnerable" "multisig:m-secure" "multisig:m-vulnerable" "nfts:n-secure" "nfts:n-vulnerable")
                local total=${#programs[@]}
                local current=0

                for prog_ver in "${programs[@]}"; do
                    IFS=':' read -r prog ver <<< "$prog_ver"
                    current=$((current + 1))
                    show_progress $current $total
                    echo ""
                    build_program "$prog" "$ver" || { print_error "Build sequence interrupted"; break; }
                done

                echo ""
                print_success "All programs built successfully!"
                ;;
            0) return ;;
            *)
                print_error "Invalid choice"
                sleep 1
                continue
                ;;
        esac

        echo ""
        echo -ne "${DIM}Press Enter to continue...${NC}"
        read
    done
}

# Handle program tests with new menu style
handle_program_tests() {
    local program=$1
    local p_secure=$2
    local p_vulnerable=$3
    local name=$4
    local secure_count=$5
    local vuln_count=$6

    show_program_menu "$name" "$secure_count" "$vuln_count"
    read choice
    echo ""

    case $choice in
        1)
            show_test_type_menu
            read test_choice
            echo ""
            case $test_choice in
                1) run_program_tests "$program" "$p_secure" "ALL" ;;
                2) run_program_tests "$program" "$p_secure" "SPECIFIC" ;;
                0) return ;;
            esac
            ;;
        2)
            show_test_type_menu
            read test_choice
            echo ""
            case $test_choice in
                1) run_program_tests "$program" "$p_vulnerable" "ALL" ;;
                2) run_program_tests "$program" "$p_vulnerable" "SPECIFIC" ;;
                0) return ;;
            esac
            ;;
        3)
            print_testing "Running secure tests..."
            echo ""
            run_program_tests "$program" "$p_secure" "ALL"
            echo ""
            print_testing "Running vulnerable tests..."
            echo ""
            run_program_tests "$program" "$p_vulnerable" "ALL"
            ;;
        0) return ;;
        *)
            print_error "Invalid choice"
            sleep 1
            return
            ;;
    esac

    echo ""
    echo -ne "${DIM}Press Enter to continue...${NC}"
    read
}

# Run all tests with progress
run_all() {
    print_section "Running All Tests"
    echo ""

    local tests=(
        "pino-escrow:p-secure:Escrow Secure"
        "pino-escrow:p-vulnerable:Escrow Vulnerable"
        "multisig:m-secure:Multisig Secure"
        "multisig:m-vulnerable:Multisig Vulnerable"
        "nfts:n-secure:NFT Secure"
        "nfts:n-vulnerable:NFT Vulnerable"
    )

    local total=${#tests[@]}
    local current=0

    for test_info in "${tests[@]}"; do
        IFS=':' read -r prog ver name <<< "$test_info"
        current=$((current + 1))

        echo ""
        show_progress $current $total
        echo ""
        print_testing "$name"
        echo ""

        run_program_tests "$prog" "$ver" "ALL"
    done

    echo ""
    print_success "All test suites completed!"
    echo ""
    echo -ne "${DIM}Press Enter to continue...${NC}"
    read
}

# Main loop
main() {
    while true; do
        show_main_menu
        read choice
        echo ""

        case $choice in
            1) handle_program_tests "pino-escrow" "p-secure" "p-vulnerable" "Pino Escrow" "4" "4" ;;
            2) handle_program_tests "multisig" "m-secure" "m-vulnerable" "Multisig" "14" "5" ;;
            3) handle_program_tests "nfts" "n-secure" "n-vulnerable" "NFT Staking" "TBD" "TBD" ;;
            4) run_all ;;
            5) handle_build ;;
            0)
                clear
                echo ""
                draw_box "Thank you for using Soteria!"
                echo ""
                echo -e "${DIM}Stay secure. Happy hacking!${NC}"
                echo ""
                exit 0
                ;;
            *)
                print_error "Invalid choice - please try again"
                sleep 1
                ;;
        esac
    done
}

# Check if we're in the right directory
if [ ! -d "programs" ]; then
    clear
    echo ""
    print_error "Must be run from the Soteria project root directory"
    echo ""
    print_info "Please cd to the project root and try again"
    echo ""
    exit 1
fi

# Start the enhanced test runner
main
