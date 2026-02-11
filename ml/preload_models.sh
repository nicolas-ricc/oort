#!/bin/bash
curl -sf http://localhost:8000/api/health > /dev/null 2>&1 || exit 1
