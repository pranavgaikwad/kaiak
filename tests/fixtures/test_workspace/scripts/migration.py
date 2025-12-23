#!/usr/bin/env python3

import os
import imp  # This is deprecated, should use importlib
import sys

def load_migration_script(path):
    """Load a migration script dynamically"""
    # Using deprecated imp module
    module = imp.load_source('migration', path)
    return module

def modern_load_script(path):
    """Modern way to load scripts"""
    import importlib.util
    spec = importlib.util.spec_from_file_location("migration", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

if __name__ == "__main__":
    print("Migration script example")