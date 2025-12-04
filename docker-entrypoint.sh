#!/bin/bash
set -e

# Create data directory if it doesn't exist
mkdir -p /app/data

# Ensure the directory is writable by appuser (UID 1000)
chown -R appuser:appuser /app/data
chmod 755 /app/data

# Switch to appuser and run the application
exec gosu appuser "$@"

