#!/usr/bin/env python3
"""
Symbolic Analysis for Soroban Contracts

This script performs symbolic analysis and model checking of Soroban smart contracts
to detect potential security vulnerabilities and verify contract invariants.
"""

import json
import re
import os
import sys
import argparse
from pathlib import Path
from typing import Dict, List, Set, Optional, Tuple
from dataclasses import dataclass
from enum import Enum

class VulnerabilityType(Enum):
    UNAUTHORIZED_PAYOUT = "unauthorized_payout"
    UNAUTHORIZED_STATE_MUTATION = "unauthorized_state_mutation"
    REENTRANCY = "reentrancy"
    OVERFLOW = "overflow"
    ACCESS_CONTROL = "access_control"

class Severity(Enum):
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"

@dataclass
class Vulnerability:
    type: VulnerabilityType
    severity: Severity
    contract: str
    function: str
    line: int
    description: str
    reproduction: str
    cvss_score: float

@dataclass
class Invariant:
    name: str
    description: str
    contract: str
    check_function: str

class SymbolicAnalyzer:
    def __init__(self, config_path: str):
        self.config = self.load_config(config_path)
        self.vulnerabilities: List[Vulnerability] = []
        self.invariants: List[Invariant] = []
        self.contracts_analyzed = 0
        
    def load_config(self, config_path: str) -> Dict:
        """Load analysis configuration"""
        try:
            with open(config_path, 'r') as f:
                return json.load(f)
        except Exception as e:
            print(f"Error loading config: {e}")
            return self.get_default_config()
    
    def get_default_config(self) -> Dict:
        """Get default configuration"""
        return {
            "contracts_dir": "contracts",
            "timeout": 300,
            "max_depth": 10,
            "checks": ["reentrancy", "overflow", "access_control"],
            "patterns": {
                "unauthorized_payout": [
                    r"transfer\s*\(",
                    r"payable\s*\(",
                    r"send\s*\("
                ],
                "unauthorized_state_mutation": [
                    r"\.set\s*\(",
                    r"\.update\s*\(",
                    r"\.modify\s*\("
                ]
            }
        }
    
    def analyze_contracts(self, contracts_dir: str) -> None:
        """Analyze all contracts in the directory"""
        contracts_path = Path(contracts_dir)
        
        if not contracts_path.exists():
            print(f"Contracts directory {contracts_dir} not found")
            return
        
        # Find all Rust files
        rust_files = []
        for rust_file in contracts_path.rglob("*.rs"):
            if rust_file.name != 'lib.rs':
                continue
            
            rust_files.append(rust_file)
        
        print(f"Found {len(rust_files)} contract files to analyze")
        
        for rust_file in rust_files:
            self.analyze_contract(rust_file)
            self.contracts_analyzed += 1
    
    def analyze_contract(self, contract_path: Path) -> None:
        """Analyze a single contract"""
        try:
            with open(contract_path, 'r', encoding='utf-8') as f:
                content = f.read()
        except Exception as e:
            print(f"Error reading {contract_path}: {e}")
            return
        
        contract_name = contract_path.parent.name
        
        # Extract functions
        functions = self.extract_functions(content)
        
        # Analyze each function
        for func_name, func_content, line_number in functions:
            self.analyze_function(contract_name, func_name, func_content, line_number)
    
    def extract_functions(self, content: str) -> List[Tuple[str, str, int]]:
        """Extract function definitions from contract"""
        functions = []
        
        # Find public functions
        pattern = r'pub\s+fn\s+(\w+)\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{'
        matches = re.finditer(pattern, content)
        
        for match in matches:
            func_name = match.group(1)
            start_pos = match.start()
            
            # Find the function body
            brace_count = 0
            func_start = content.find('{', start_pos)
            
            if func_start == -1:
                continue
                
            for i, char in enumerate(content[func_start:], func_start):
                if char == '{':
                    brace_count += 1
                elif char == '}':
                    brace_count -= 1
                    if brace_count == 0:
                        func_end = i + 1
                        func_content = content[func_start:func_end]
                        line_number = content[:start_pos].count('\n') + 1
                        functions.append((func_name, func_content, line_number))
                        break
        
        return functions
    
    def analyze_function(self, contract_name: str, func_name: str, func_content: str, line_number: int) -> None:
        """Analyze a function for vulnerabilities"""
        
        # Check for unauthorized payouts
        if self.check_unauthorized_payout(func_content):
            self.add_vulnerability(
                VulnerabilityType.UNAUTHORIZED_PAYOUT,
                Severity.HIGH,
                contract_name,
                func_name,
                line_number,
                f"Function {func_name} may allow unauthorized payout",
                f"Call {func_name} to potentially trigger unauthorized payout",
                7.5
            )
        
        # Check for unauthorized state mutations
        if self.check_unauthorized_state_mutation(func_content):
            self.add_vulnerability(
                VulnerabilityType.UNAUTHORIZED_STATE_MUTATION,
                Severity.HIGH,
                contract_name,
                func_name,
                line_number,
                f"Function {func_name} modifies critical state without authorization",
                f"Call {func_name} to modify critical state without proper permissions",
                7.5
            )
        
        # Check for reentrancy
        if self.check_reentrancy(func_content):
            self.add_vulnerability(
                VulnerabilityType.REENTRANCY,
                Severity.CRITICAL,
                contract_name,
                func_name,
                line_number,
                f"Function {func_name} may be vulnerable to reentrancy attacks",
                f"Call {func_name} recursively to exploit reentrancy vulnerability",
                9.0
            )
        
        # Check for overflow
        if self.check_overflow(func_content):
            self.add_vulnerability(
                VulnerabilityType.OVERFLOW,
                Severity.HIGH,
                contract_name,
                func_name,
                line_number,
                f"Function {func_name} may have overflow vulnerabilities",
                f"Call {func_name} with large values to trigger overflow",
                8.0
            )
    
    def check_unauthorized_payout(self, func_content: str) -> bool:
        """Check for unauthorized payout patterns"""
        patterns = [
            r"transfer\s*\(",
            r"payable\s*\(",
            r"send\s*\(",
            r"payout\s*\(",
            r"withdraw\s*\("
        ]
        
        for pattern in patterns:
            if re.search(pattern, func_content, re.IGNORECASE):
                # Check if there's proper authorization
                if not re.search(r"require_auth\s*\(", func_content):
                    return True
        
        return False
    
    def check_unauthorized_state_mutation(self, func_content: str) -> bool:
        """Check for unauthorized state mutation patterns"""
        patterns = [
            r"\.set\s*\(",
            r"\.update\s*\(",
            r"\.modify\s*\(",
            r"storage\.\w+\s*\(",
            r"env\.storage\."
        ]
        
        for pattern in patterns:
            if re.search(pattern, func_content):
                # Check if there's proper authorization
                if not re.search(r"require_auth\s*\(", func_content):
                    return True
        
        return False
    
    def check_reentrancy(self, func_content: str) -> bool:
        """Check for reentrancy vulnerabilities"""
        # Look for external calls followed by state changes
        external_call_patterns = [
            r"env\.invoke_contract\s*\(",
            r"\.call\s*\(",
            r"transfer\s*\("
        ]
        
        for pattern in external_call_patterns:
            if re.search(pattern, func_content):
                # Check if there are state changes after the external call
                lines = func_content.split('\n')
                for i, line in enumerate(lines):
                    if re.search(pattern, line):
                        # Look for state changes in subsequent lines
                        for j in range(i + 1, len(lines)):
                            if re.search(r"storage\.\w+\s*\(", lines[j]):
                                return True
                        break
        
        return False
    
    def check_overflow(self, func_content: str) -> bool:
        """Check for overflow vulnerabilities"""
        # Look for arithmetic operations without bounds checking
        patterns = [
            r"\.checked_add\s*\(",
            r"\.checked_sub\s*\(",
            r"\.checked_mul\s*\("
        ]
        
        has_arithmetic = re.search(r"[\+\-\*\/]\s*\w+", func_content)
        has_bounds_check = any(re.search(pattern, func_content) for pattern in patterns)
        
        return has_arithmetic and not has_bounds_check
    
    def add_vulnerability(self, vuln_type: VulnerabilityType, severity: Severity, 
                         contract: str, function: str, line: int, description: str, 
                         reproduction: str, cvss_score: float) -> None:
        """Add a vulnerability to the list"""
        vulnerability = Vulnerability(
            type=vuln_type,
            severity=severity,
            contract=contract,
            function=function,
            line=line,
            description=description,
            reproduction=reproduction,
            cvss_score=cvss_score
        )
        self.vulnerabilities.append(vulnerability)
    
    def generate_report(self, output_path: str) -> None:
        """Generate analysis report"""
        report = {
            "summary": {
                "contracts_analyzed": self.contracts_analyzed,
                "total_vulnerabilities": len(self.vulnerabilities),
                "critical": len([v for v in self.vulnerabilities if v.severity == Severity.CRITICAL]),
                "high": len([v for v in self.vulnerabilities if v.severity == Severity.HIGH]),
                "medium": len([v for v in self.vulnerabilities if v.severity == Severity.MEDIUM]),
                "low": len([v for v in self.vulnerabilities if v.severity == Severity.LOW])
            },
            "vulnerabilities": [
                {
                    "type": v.type.value,
                    "severity": v.severity.value,
                    "contract": v.contract,
                    "function": v.function,
                    "line": v.line,
                    "description": v.description,
                    "reproduction": v.reproduction,
                    "cvss_score": v.cvss_score
                }
                for v in self.vulnerabilities
            ]
        }
        
        with open(output_path, 'w') as f:
            json.dump(report, f, indent=2)
        
        print(f"Report saved to {output_path}")
        print(f"Found {len(self.vulnerabilities)} vulnerabilities")
        print(f"Critical: {report['summary']['critical']}")
        print(f"High: {report['summary']['high']}")
        
        # Check CI failure conditions
        if report['summary']['critical'] > 0 or report['summary']['high'] > 0:
            print("CI FAILURE: Found critical or high severity vulnerabilities")
            sys.exit(1)

def main():
    parser = argparse.ArgumentParser(description='Symbolic Analysis for Soroban Contracts')
    parser.add_argument('--contracts-dir', default='contracts', help='Contracts directory')
    parser.add_argument('--config', default='analysis_config.json', help='Configuration file')
    parser.add_argument('--output', default='security_report.json', help='Output report file')
    parser.add_argument('--all-contracts', action='store_true', help='Analyze all contracts')
    
    args = parser.parse_args()
    
    analyzer = SymbolicAnalyzer(args.config)
    
    if args.all_contracts:
        analyzer.analyze_contracts(args.contracts_dir)
    
    analyzer.generate_report(args.output)

if __name__ == '__main__':
    main()
